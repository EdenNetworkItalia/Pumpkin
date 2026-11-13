use crate::{
    ServerPacket,
    ser::{NetworkReadExt, NetworkReadSliceExt, ReadingError},
};
use pumpkin_data::packet::serverbound::play::CLIENT_INFORMATION;
use pumpkin_macros::java_packet;
use pumpkin_util::version::JavaMinecraftVersion;

use crate::VarInt;

#[java_packet(CLIENT_INFORMATION)]
pub struct SClientInformationPlay<'a> {
    pub locale: &'a str, // 16
    pub view_distance: i8,
    pub chat_mode: VarInt, // VarInt
    pub chat_colors: bool,
    pub skin_parts: u8,
    pub main_hand: VarInt,
    pub text_filtering: bool,
    pub server_listing: bool,
    pub particle_status: VarInt,
}

impl<'a> ServerPacket<'a> for SClientInformationPlay<'a> {
    fn read(bytebuf: &mut &'a [u8], version: &JavaMinecraftVersion) -> Result<Self, ReadingError> {
        let locale = bytebuf.get_str_borrowed()?;
        let view_distance = bytebuf.get_i8()?;
        let chat_mode = bytebuf.get_var_int()?;
        let chat_colors = bytebuf.get_bool()?;
        let skin_parts = bytebuf.get_u8()?;
        let main_hand = if version >= &JavaMinecraftVersion::V_1_9 {
            bytebuf.get_var_int()?
        } else {
            VarInt(1)
        };
        let text_filtering = if version >= &JavaMinecraftVersion::V_1_17 {
            bytebuf.get_bool()?
        } else {
            false
        };
        let server_listing = if version >= &JavaMinecraftVersion::V_1_18 {
            bytebuf.get_bool()?
        } else {
            true
        };
        let particle_status = if version >= &JavaMinecraftVersion::V_1_21_2 {
            bytebuf.get_var_int()?
        } else {
            VarInt(0)
        };

        Ok(Self {
            locale,
            view_distance,
            chat_mode,
            chat_colors,
            skin_parts,
            main_hand,
            text_filtering,
            server_listing,
            particle_status,
        })
    }
}

impl crate::ClientPacket for SClientInformationPlay<'_> {
    fn write_packet_data(
        &self,
        mut write: impl std::io::Write,
        version: &JavaMinecraftVersion,
    ) -> Result<(), crate::ser::WritingError> {
        use crate::ser::NetworkWriteExt;
        write.write_string(self.locale)?;
        write.write_i8(self.view_distance)?;
        write.write_var_int(&self.chat_mode)?;
        write.write_bool(self.chat_colors)?;
        write.write_u8(self.skin_parts)?;
        if version >= &JavaMinecraftVersion::V_1_9 {
            write.write_var_int(&self.main_hand)?;
        }
        if version >= &JavaMinecraftVersion::V_1_17 {
            write.write_bool(self.text_filtering)?;
        }
        if version >= &JavaMinecraftVersion::V_1_18 {
            write.write_bool(self.server_listing)?;
        }
        if version >= &JavaMinecraftVersion::V_1_21_2 {
            write.write_var_int(&self.particle_status)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ClientPacket;
    use crate::java::server::config::SClientInformationConfig;

    /// `ClientInformation.write` output for `en_us`, view distance 12, chat `SYSTEM`, no chat
    /// colors, skin parts 0x5a, left hand, text filtering on, no listing, particles `MINIMAL`.
    /// The configuration and play packets share this layout.
    const VANILLA: [u8; 14] = [
        0x05, b'e', b'n', b'_', b'u', b's', 0x0c, 0x01, 0x00, 0x5a, 0x00, 0x01, 0x00, 0x02,
    ];

    #[test]
    fn reads_vanilla_client_information() {
        let version = JavaMinecraftVersion::V_26_3;

        let mut bytes = &VANILLA[..];
        let play = SClientInformationPlay::read(&mut bytes, &version).unwrap();
        assert!(bytes.is_empty(), "{} bytes left unread", bytes.len());
        assert_eq!(play.particle_status, VarInt(2));
        let mut written = Vec::new();
        play.write_packet_data(&mut written, &version).unwrap();
        assert_eq!(written, VANILLA);

        let mut bytes = &VANILLA[..];
        let config = SClientInformationConfig::read(&mut bytes, &version).unwrap();
        assert!(bytes.is_empty(), "{} bytes left unread", bytes.len());
        assert_eq!(config.particle_status, VarInt(2));
        let mut written = Vec::new();
        config.write_packet_data(&mut written, &version).unwrap();
        assert_eq!(written, VANILLA);
    }
}
