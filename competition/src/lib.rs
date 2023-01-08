use cursive::align::HAlign;
use cursive::event::Event;
use cursive::traits::{Nameable, Scrollable};
use cursive::views::{Dialog, SelectView, TextView};
use cursive::Cursive;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use v5_core::clap::{value_parser, Arg, ArgMatches, Command};
use v5_core::error::Error;
use v5_core::export_plugin;
use v5_core::log::error;
use v5_core::plugin::{Plugin, PORT};
use v5_core::serial::system::{Brain, CompetitionStatus};
use v5_core::tokio::sync::Notify;
use v5_core::tokio::task;

type Result<T> = std::result::Result<T, Error>;

const COMPETITION: &str = "competition";

const START: &str = "status";
const DISABLE: &str = "metadata";
const AUTONOMOUS: &str = "ls_files";
const OPCONTROL: &str = "file_name";
const LENGTH: &str = "vid";

export_plugin!(Box::new(CompetitionPlugin::default()));

pub struct CompetitionPlugin {}

impl Default for CompetitionPlugin {
    fn default() -> Self {
        CompetitionPlugin {}
    }
}

impl Plugin for CompetitionPlugin {
    fn get_name(&self) -> &'static str {
        COMPETITION
    }

    fn create_commands(
        &self,
        command: Command,
        registry: &mut HashMap<
            &'static str,
            Box<fn(ArgMatches) -> Pin<Box<dyn Future<Output = ()>>>>,
        >,
    ) -> Command {
        registry.insert(COMPETITION, Box::new(|f| Box::pin(competition(f))));
        command.subcommand(
            Command::new(COMPETITION)
                .about("Simulate a competition")
                .subcommand(Command::new(START).about("Starts an interactive competition manager"))
                .subcommand(
                    Command::new(AUTONOMOUS)
                        .about("Runs the autonomous period, then disables the robot")
                        .arg(
                            Arg::new(LENGTH)
                                .short('l')
                                .default_value("15000")
                                .value_parser(value_parser!(u64)),
                        ),
                )
                .subcommand(
                    Command::new(OPCONTROL)
                        .about("Runs the operator control period, then disables the robot")
                        .arg(
                            Arg::new(LENGTH)
                                .short('l')
                                .default_value("105000")
                                .value_parser(value_parser!(u64)),
                        ),
                )
                .subcommand(Command::new(DISABLE).about("Disables the robot")),
        )
    }
}

async fn competition(args: ArgMatches) {
    let mut brain =
        v5_core::serial::connect_to_brain(args.get_one(PORT).map(|f: &String| f.to_string()));
    if let Some((command, args)) = args.subcommand() {
        match command {
            START => start(brain, args).await,
            AUTONOMOUS => autonomous(brain, args).await,
            OPCONTROL => opcontrol(brain, args).await,
            DISABLE => disable(brain, args).await,
            _ => Err(Error::Generic("Invalid subcommand! (see `--help`)")),
        }
        .unwrap()
    } else {
        error!("Missing subcommand (see `--help`)");
    }
}

async fn autonomous(mut brain: Brain, args: &ArgMatches) -> Result<()> {
    let time = Duration::from_millis(*args.get_one::<u64>(LENGTH).expect("length"));
    brain.manage_competition(CompetitionStatus::Autonomous)?;
    std::thread::sleep(time);
    Ok(())
}

async fn opcontrol(mut brain: Brain, args: &ArgMatches) -> Result<()> {
    let time = Duration::from_millis(*args.get_one::<u64>(LENGTH).expect("length"));
    brain.manage_competition(CompetitionStatus::OpControl)?;
    std::thread::sleep(time);
    Ok(())
}

async fn disable(mut brain: Brain, args: &ArgMatches) -> Result<()> {
    brain.manage_competition(CompetitionStatus::Disabled)?;
    Ok(())
}

const INFINITE: Duration = Duration::from_secs(u64::MAX);

#[derive(Clone)]
struct State {
    control_type: CompetitionStatus,
    duration: Option<Duration>,
    start_time: SystemTime,
}

impl State {
    fn new(
        control_type: CompetitionStatus,
        duration: Option<Duration>,
        start_time: SystemTime,
    ) -> Self {
        if matches!(control_type, CompetitionStatus::Disabled) {
            assert!(duration.is_none())
        }

        State {
            control_type,
            duration,
            start_time,
        }
    }
}

static STATE: AtomicU8 = AtomicU8::new(0);

async fn start(mut brain: Brain, args: &ArgMatches) -> Result<()> {
    let notify = Arc::new(Notify::new());
    let logic_notify = notify.clone();

    let notify_1 = notify.clone(); // i wish tokio had structured concurrency
    let notify_2 = notify.clone();
    let notify_3 = notify.clone();
    let notify_4 = notify.clone();
    let notify_5 = notify.clone();
    let notify_6 = notify.clone();

    let logic_task = task::spawn(async move {
        let mut prev_state = 255_u8;
        loop {
            let state = STATE.load(Ordering::Relaxed);
            if prev_state != state {
                prev_state = state;

                let control_type = CompetitionStatus::try_from(state).unwrap();
                match control_type {
                    CompetitionStatus::Disabled => brain
                        .manage_competition(CompetitionStatus::Disabled)
                        .unwrap(),
                    CompetitionStatus::Autonomous => brain
                        .manage_competition(CompetitionStatus::Autonomous)
                        .unwrap(),
                    CompetitionStatus::OpControl => brain
                        .manage_competition(CompetitionStatus::OpControl)
                        .unwrap(),
                }
            }
            logic_notify.notified().await;
        }
    });

    let mut gui = cursive::crossterm();
    gui.set_user_data(State::new(
        CompetitionStatus::Disabled,
        None,
        SystemTime::now(),
    ));
    gui.add_layer(
        Dialog::around(TextView::new("---").center().with_name("main_text"))
            .title("v5_cli Competition Manager")
            .button("Autonomous", move |f| autonomous_button(f, &notify_4))
            .button("OpControl", move |f| opcontrol_button(f, &notify_5))
            .button("Disable", move |f| disable_button(f, &notify_6))
            .button("Quit", |f| f.quit()),
    );
    gui.add_global_callback(Event::Exit, move |_| logic_task.abort());
    gui.add_global_callback(Event::Char('a'), move |f| autonomous_button(f, &notify_1));
    gui.add_global_callback(Event::Char('o'), move |f| opcontrol_button(f, &notify_2));
    gui.add_global_callback(Event::Char('d'), move |f| disable_button(f, &notify_3));
    gui.add_global_callback(Event::Char('q'), |f| f.quit());
    gui.add_global_callback(Event::Refresh, update_timer);
    gui.run();

    Ok(())
}

fn update_timer(gui: &mut Cursive) {
    let x: Option<&mut State> = gui.user_data();
    if let Some(state) = x {
        let state = state.clone(); //fixme
        match state.control_type {
            CompetitionStatus::Autonomous => {
                gui.call_on_name("main_text", |view: &mut TextView| {
                    let duration = state.start_time.elapsed().unwrap();
                    let elapsed_seconds = duration.as_secs() % 60;
                    let elapsed_minutes = duration.as_secs() / 60;
                    if let Some(time) = state.duration {
                        let total_seconds = time.as_secs() % 60;
                        let total_minutes = time.as_secs() / 60;
                        view.set_content(format!(
                            "Autonomous\n{}:{:02} / {}:{:02}",
                            elapsed_minutes, elapsed_seconds, total_minutes, total_seconds
                        ));
                    } else {
                        view.set_content(format!(
                            "Autonomous\n{}:{:02}",
                            elapsed_minutes, elapsed_seconds
                        ));
                    }
                });
            }
            CompetitionStatus::OpControl => {
                gui.call_on_name("main_text", |view: &mut TextView| {
                    let duration = state.start_time.elapsed().unwrap();
                    let elapsed_seconds = duration.as_secs() % 60;
                    let elapsed_minutes = duration.as_secs() / 60;
                    if let Some(time) = state.duration {
                        let total_seconds = time.as_secs() % 60;
                        let total_minutes = time.as_secs() / 60;
                        view.set_content(format!(
                            "Operator Control\n{}:{:02} / {}:{:02}",
                            elapsed_minutes, elapsed_seconds, total_minutes, total_seconds
                        ));
                    } else {
                        view.set_content(format!(
                            "Operator Control\n{}:{:02}",
                            elapsed_minutes, elapsed_seconds
                        ));
                    }
                });
            }
            CompetitionStatus::Disabled => {
                gui.call_on_name("main_text", |view: &mut TextView| {
                    view.set_content("Disabled")
                });
            }
        }
    }
}

fn autonomous_button(gui: &mut Cursive, listener: &Arc<Notify>) {
    let mut view = SelectView::new().h_align(HAlign::Center);
    view.add_item("0:15", Duration::from_secs(15));
    view.add_item("1:00", Duration::from_secs(60));
    view.add_item("infinite", INFINITE);

    let listener = listener.clone();

    view.set_on_submit(move |gui: &mut Cursive, value: &Duration| {
        gui.pop_layer();

        gui.set_user_data(State::new(
            CompetitionStatus::Autonomous,
            if value == &INFINITE {
                None
            } else {
                Some(*value)
            },
            SystemTime::now(),
        ));
        STATE.store(CompetitionStatus::Autonomous as u8, Ordering::Relaxed);
        listener.notify_one();
    });

    gui.add_layer(Dialog::around(view.scrollable()));
}

fn opcontrol_button(gui: &mut Cursive, listener: &Arc<Notify>) {
    let mut view = SelectView::new().h_align(HAlign::Center);
    view.add_item("1:45", Duration::from_secs(105));
    view.add_item("1:00", Duration::from_secs(60));
    view.add_item("infinite", INFINITE);

    let listener = listener.clone();

    view.set_on_submit(move |gui: &mut Cursive, value: &Duration| {
        gui.pop_layer();

        gui.set_user_data(State::new(
            CompetitionStatus::OpControl,
            if value == &INFINITE {
                None
            } else {
                Some(*value)
            },
            SystemTime::now(),
        ));
        STATE.store(CompetitionStatus::OpControl as u8, Ordering::Relaxed);
        listener.notify_one();
    });

    gui.add_layer(Dialog::around(view.scrollable()));
}

fn disable_button(gui: &mut Cursive, listener: &Arc<Notify>) {
    gui.set_user_data(State::new(
        CompetitionStatus::Disabled,
        None,
        SystemTime::now(),
    ));
    STATE.store(CompetitionStatus::Disabled as u8, Ordering::Relaxed);
    listener.notify_one();
}
