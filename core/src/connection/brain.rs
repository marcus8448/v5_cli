use std::time::{Duration, SystemTime};
use std::mem::size_of;
use async_recursion::async_recursion;
use std::sync::atomic::Ordering;
use crate::buffer::{FixedReadBuffer, OwnedBuffer, RawWrite};
use crate::connection::{CRC16, EXT_PACKET_ID, Nack, PACKET_HEADER, PACKETS_LOST, RESPONSE_HEADER, SerialConnection, TIMEOUT};
use crate::packet::{Packet, PacketBuf, PacketType};

pub struct Brain {
    connection: Box<dyn SerialConnection + Send>
}

impl Brain {
    pub fn new(connection: Box<dyn SerialConnection + Send>) -> Self {
        Self { connection }
    }

    pub fn packet(&mut self, content_len: u16, packet_type: PacketType) -> PacketBuf {
        PacketBuf::new(packet_type, content_len, self)
    }

    pub async fn send_raw_packet(&mut self, data: &[u8]) -> Result<(), std::io::Error> {
        self.connection.clear().await?;
        self.connection.write_all(data).await?;
        self.connection.flush().await?;
        Ok(())
    }

    pub async fn find_packet_header(&mut self) -> Result<bool, std::io::Error> {
        let mut value = 0;
        let mut i = 0;
        let time = SystemTime::now();
        loop {
            if value == RESPONSE_HEADER[i] {
                i += 1;
                if i == RESPONSE_HEADER.len() {
                    break;
                }
            } else if i > 0 {
                i = 0;
                continue;
            }

            match self.connection.try_read_one().await {
                Ok(v) => value = v,
                Err(_) => {
                    tokio::time::sleep(Duration::from_millis(5)).await;
                    value = 0;
                    if SystemTime::now()
                        .duration_since(time)
                        .unwrap_or(Duration::ZERO)
                        > TIMEOUT {
                        return Ok(false);
                    }
                }
            }
        }
        println!(
            "response took {}ms",
            SystemTime::now().duration_since(time).unwrap().as_millis()
        );
        Ok(true)
    }

    pub async fn receive_raw_packet(&mut self, id: u8) -> Result<OwnedBuffer, std::io::Error> {
        loop {
            match self.find_packet_header().await {
                Ok(true) => {
                    break
                }
                Ok(false) => {
                    return Err(std::io::ErrorKind::TimedOut.into())
                }
                _ => {
                    return Err(std::io::ErrorKind::UnexpectedEof.into())
                }
            };
        }

        let mut payload = Vec::with_capacity(64);
        payload.extend_from_slice(&RESPONSE_HEADER);

        let mut metadata = [0_u8; 2];

        self.connection.read(&mut metadata).await?;
        let command = metadata[0];
        let mut len: usize = metadata[1] as usize;

        payload.extend_from_slice(&metadata);

        if len & 0x80 == 0x80 {
            let val = self.connection.try_read_one().await?;
            len = ((len & 0x7f) << 8) + val as usize;
            payload.push(val);
        }

        let start = payload.len();
        payload.reserve(len);
        payload.resize(start + len, 0_u8);

        self.connection.read(&mut payload[start..]).await?;

        assert_eq!(command, EXT_PACKET_ID);
        assert_eq!(id, payload[start]);
        assert_eq!(CRC16.checksum(&payload), 0);

        if let Ok(nack) = Nack::try_from(payload[start + 1]) {
            println!("NACK: {:?} ({})", &nack, payload[start + 1]);
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "NACK"));
        }

        Ok(OwnedBuffer::new(payload.into_boxed_slice(), (start + 2) as u16))
    }

    pub async fn send_simple(
        &mut self,
        id: u8
    ) -> Result<OwnedBuffer, std::io::Error> {
        let mut buffer = [0_u8; 4 + 1 + /*CRC*/ size_of::<u16>()];
        buffer[0..PACKET_HEADER.len()].copy_from_slice(PACKET_HEADER);
        buffer[PACKET_HEADER.len()] = id;

        self.connection.clear().await?;
        self.connection.write_all(&buffer).await?;
        self.connection.flush().await?;

        let mut value = 0;
        let mut i = 0;
        let time = SystemTime::now();

        loop {
            if value == RESPONSE_HEADER[i] {
                i += 1;
                if i == RESPONSE_HEADER.len() {
                    break;
                }
            } else if i > 0 {
                i = 0;
                continue;
            }

            match self.connection.try_read_one().await {
                Ok(v) => value = v,
                Err(_) => {
                    tokio::time::sleep(Duration::from_millis(5)).await;
                    value = 0;
                    if SystemTime::now()
                        .duration_since(time)
                        .unwrap_or(Duration::ZERO)
                        > TIMEOUT {
                        return Err(std::io::ErrorKind::TimedOut.into());
                    }
                }
            }
        }
        println!(
            "response took {}ms",
            SystemTime::now().duration_since(time).unwrap().as_millis()
        );

        let mut payload = Vec::with_capacity(64);
        payload.extend_from_slice(&RESPONSE_HEADER);

        let mut metadata = [0_u8; 2];

        self.connection.read(&mut metadata).await?;
        let command = self.connection.try_read_one().await?;
        let len = self.connection.try_read_one().await? as usize;

        payload.extend_from_slice(&metadata);

        let start = payload.len();
        payload.reserve(len);
        payload.resize(start + len, 0_u8);

        self.connection.read(&mut payload[start..]).await?;

        assert_eq!(command, id);

        Ok(OwnedBuffer::new(payload.into_boxed_slice(), (start + 1) as u16))
    }

    #[async_recursion(?Send)]
    pub async fn send<const ID: u8, T>(
        &mut self,
        packet: &mut dyn Packet<ID, Response=T>,
    ) -> std::result::Result<T, std::io::Error> {
        dbg!(&packet);
        let len = packet.send_len();
        let mut buffer =
            Vec::with_capacity(4 + 1 + 1 + if len < 0x80 { 1 } else { 2 } + len + size_of::<u16>());
        buffer.write_raw(PACKET_HEADER);

        if packet.is_simple() {
            buffer.write_u8(ID);
        } else {
            buffer.write_u8(EXT_PACKET_ID);
            buffer.write_u8(ID);

            if len < 0x80 {
                println!("normal size {}", len);
                buffer.write_u8(len as u8);
            } else {
                println!("pack size {}", len);
                buffer.write_u8((len >> 8 | 0x80) as u8);
                buffer.write_u8((len & 0xff) as u8);
            }

            let i = buffer.len();

            packet.write_buffer(&mut buffer)?;
            let j = buffer.len();
            println!("Act size: {}", j - i);

            buffer.write_raw(&CRC16.checksum(&buffer).to_be_bytes());
        }

        self.connection.clear().await?;
        self.connection.write_all(&buffer).await?;
        self.connection.flush().await?;

        let mut value = 0;
        let mut i = 0;
        let time = SystemTime::now();
        loop {
            if value == RESPONSE_HEADER[i] {
                i += 1;
                if i == RESPONSE_HEADER.len() {
                    break;
                }
            } else if i > 0 {
                i = 0;
                continue;
            }

            match self.connection.try_read_one().await {
                Ok(v) => value = v,
                Err(_) => {
                    tokio::time::sleep(Duration::from_millis(5)).await;
                    value = 0;
                    let mut dur = Duration::from_millis(300);
                    if ID == 0x12 {
                        dur = Duration::from_millis(2000);
                    }
                    if SystemTime::now()
                        .duration_since(time)
                        .expect("time ran backwards")
                        > dur
                    {
                        println!(
                            "resending ----------------------------------- {}",
                            PACKETS_LOST.fetch_add(1, Ordering::Relaxed) + 1
                        );
                        return self.send(packet).await;
                    }
                }
            }
        }
        println!(
            "response took {}ms",
            SystemTime::now().duration_since(time).unwrap().as_millis()
        );

        let mut payload = Vec::with_capacity(64);
        payload.extend_from_slice(&RESPONSE_HEADER);

        let mut metadata = [0_u8; 2];

        self.connection.read(&mut metadata).await?;
        let command = metadata[0];
        let mut len: usize = metadata[1] as usize;

        payload.extend_from_slice(&metadata);

        if !packet.is_simple() && len & 0x80 != 0 {
            let val = self.connection.try_read_one().await?;
            len = ((len & 0x7f) << 8) + val as usize;
            payload.push(val);
        }

        let start = payload.len();
        payload.reserve(len);
        payload.resize(start + len, 0_u8);

        self.connection.read(&mut payload[start..]).await?;

        // println!(
        //     "received data ({}): {:02X?}",
        //     len - if packet::is_simple() { 1 } else { 4 },
        //     &payload
        // );

        if packet.is_simple() {
            assert_eq!(command, ID);
            Ok(packet.read_response(&mut FixedReadBuffer::new(&payload[start + 1..]), len - 1)?)
        } else {
            assert_eq!(command, EXT_PACKET_ID);
            assert_eq!(ID, payload[start]);
            assert_eq!(CRC16.checksum(&payload), 0);

            if let Ok(nack) = Nack::try_from(payload[start + 1]) {
                println!("NACK: {:?} ({})", &nack, payload[start + 1]);
                return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "NACK"));
            }
            Ok(packet.read_response(
                &mut FixedReadBuffer::new(&payload[start + 2..payload.len() - 2]),
                len - 4,
            )?)
        }
    }
}
