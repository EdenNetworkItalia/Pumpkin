use pumpkin_data::packet::clientbound::config::DISCONNECT;
use pumpkin_macros::java_packet;
use pumpkin_util::text::TextComponent;

use crate::ClientPacket;
use crate::ser::NetworkWriteExt;
use pumpkin_util::version::JavaMinecraftVersion;

#[java_packet(DISCONNECT)]
pub struct CConfigDisconnect<'a> {
    pub reason: &'a TextComponent,
}

impl<'a> CConfigDisconnect<'a> {
    #[must_use]
    pub const fn new(reason: &'a TextComponent) -> Self {
        Self { reason }
    }
}

impl ClientPacket for CConfigDisconnect<'_> {
    fn write_packet_data(
        &self,
        mut write: impl std::io::Write,
        version: &JavaMinecraftVersion,
    ) -> Result<(), crate::ser::WritingError> {
        write.write_component(self.reason, version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ser::NetworkReadSliceExt;
    use pumpkin_data::{packet::CURRENT_MC_VERSION, translation};
    use pumpkin_nbt::tag::NbtTag;

    #[test]
    fn translatable_reason_is_network_nbt_with_key_and_argument() {
        let reason = pumpkin_macros::translate_cross!(
            translation::java::MULTIPLAYER_PLAYER_JOINED,
            translation::bedrock::MULTIPLAYER_PLAYER_JOINED,
            TextComponent::text("Steve")
        );
        let mut data = Vec::new();
        CConfigDisconnect::new(&reason)
            .write_packet_data(&mut data, &CURRENT_MC_VERSION)
            .unwrap();

        let mut read = data.as_slice();
        let Some(NbtTag::Compound(compound)) = read.get_nbt(&CURRENT_MC_VERSION).unwrap() else {
            panic!("reason is not an NBT compound: {data:02x?}");
        };
        assert!(read.is_empty());
        assert_eq!(
            compound.get_string("translate"),
            Some(translation::java::MULTIPLAYER_PLAYER_JOINED)
        );
        let Some([NbtTag::String(argument)]) = compound.get_list("with") else {
            panic!("unexpected with list: {compound:?}");
        };
        assert_eq!(&**argument, "Steve");
    }
}
