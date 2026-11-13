use std::collections::HashMap;

use pumpkin_data::{Block, BlockState, BlockStateId, Property};
use pumpkin_nbt::compound::NbtCompound;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// `StateHolder.ID_TAG`.
const ID_TAG: &str = "id";
/// `StateHolder.PROPERTIES_TAG`.
const PROPERTIES_TAG: &str = "properties";
/// The keys of 26.2 and older. `BlockStateFieldNamesFix` renames them to the two above.
const LEGACY_ID_TAG: &str = "Name";
const LEGACY_PROPERTIES_TAG: &str = "Properties";

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct BlockStateCodec {
    /// Block name
    #[serde(
        deserialize_with = "parse_block_name",
        serialize_with = "block_to_string"
    )]
    pub name: &'static Block,
    /// Key-value pairs of properties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, String>>,
}

fn parse_block_name<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<&'static Block, D::Error> {
    let s = String::deserialize(deserializer)?;
    let block =
        Block::from_name(s.as_str()).ok_or(serde::de::Error::custom("Invalid block name"))?;
    Ok(block)
}

fn block_to_string<S: Serializer>(block: &'static Block, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(block.name)
}

impl BlockStateCodec {
    #[must_use]
    pub fn get_state(&self) -> &'static BlockState {
        let state_id = self.get_state_id();
        BlockState::from_id(state_id)
    }

    #[must_use]
    pub const fn get_block(&self) -> &'static Block {
        self.name
    }

    /// Prefer this over `get_state` when the only the state ID is needed
    #[must_use]
    pub fn get_state_id(&self) -> BlockStateId {
        let block = self.name;

        let Some(properties_map) = &self.properties else {
            return block.default_state.id;
        };

        let props_iter = properties_map
            .iter()
            .filter_map(|(k, v)| Property::from_str_pair(k, v))
            .collect::<Vec<Property>>();

        let block_properties = block.from_properties(&props_iter);
        block_properties.to_state_id(block)
    }

    /// Writes `id`, plus `properties` when the state has any.
    #[must_use]
    pub fn state_to_nbt(state_id: BlockStateId) -> NbtCompound {
        let block = Block::from_state_id(state_id);
        let mut nbt = NbtCompound::new();
        nbt.put_string(ID_TAG, format!("minecraft:{}", block.name));
        if let Some(properties) = block.properties(state_id) {
            let properties = properties.to_props();
            if !properties.is_empty() {
                let mut properties_nbt = NbtCompound::new();
                for property in properties {
                    properties_nbt.put_string(property.key_str(), property.value_str().to_string());
                }
                nbt.put_compound(PROPERTIES_TAG, properties_nbt);
            }
        }
        nbt
    }

    /// Reads a state written by [`Self::state_to_nbt`], or by 26.2 and older with `Name` and
    /// `Properties`. Unknown blocks give `None`.
    #[must_use]
    pub fn state_from_nbt(nbt: &NbtCompound) -> Option<BlockStateId> {
        let name = nbt
            .get_string(ID_TAG)
            .or_else(|| nbt.get_string(LEGACY_ID_TAG))?;
        let block = Block::from_name(name)?;
        let Some(properties) = nbt
            .get_compound(PROPERTIES_TAG)
            .or_else(|| nbt.get_compound(LEGACY_PROPERTIES_TAG))
        else {
            return Some(block.default_state.id);
        };
        let properties = properties
            .child_tags
            .iter()
            .filter_map(|(key, value)| Property::from_str_pair(key, value.extract_string()?))
            .collect::<Vec<Property>>();
        Some(block.from_properties(&properties).to_state_id(block))
    }
}

#[cfg(test)]
mod test {
    use pumpkin_data::BlockStateId;

    use crate::chunk::palette::BLOCK_NETWORK_MAX_BITS;

    #[test]
    fn block_state_nbt_matches_vanilla_layout() {
        use super::BlockStateCodec;
        use pumpkin_data::Block;

        // 26.3 `StateHolder.codec`: `id` and `properties`.
        let state = Block::GRASS_BLOCK.default_state.id;
        let nbt = BlockStateCodec::state_to_nbt(state);
        assert_eq!(nbt.get_string("id"), Some("minecraft:grass_block"));
        assert_eq!(
            nbt.get_compound("properties")
                .and_then(|properties| properties.get_string("snowy")),
            Some("false")
        );
        assert!(nbt.get_string("Name").is_none());
        assert!(nbt.get_compound("Properties").is_none());
        assert_eq!(BlockStateCodec::state_from_nbt(&nbt), Some(state));

        let stone = BlockStateCodec::state_to_nbt(Block::STONE.default_state.id);
        assert!(stone.get_compound("properties").is_none());
    }

    #[test]
    fn block_state_nbt_reads_the_26_2_keys() {
        use super::BlockStateCodec;
        use pumpkin_data::Block;
        use pumpkin_nbt::compound::NbtCompound;

        let mut properties = NbtCompound::new();
        properties.put_string("snowy", "true".to_string());
        let mut nbt = NbtCompound::new();
        nbt.put_string("Name", "minecraft:grass_block".to_string());
        nbt.put_compound("Properties", properties);

        let expected = Block::GRASS_BLOCK
            .from_properties(&[pumpkin_data::Property::Snowy(true)])
            .to_state_id(&Block::GRASS_BLOCK);
        assert_eq!(BlockStateCodec::state_from_nbt(&nbt), Some(expected));
    }

    #[test]
    fn proper_network_bits_per_entry() {
        let addressable = 1u32 << BLOCK_NETWORK_MAX_BITS;
        assert!(
            u32::from(BlockStateId::COUNT) <= addressable,
            "We need to update our constants! {} states do not fit in {BLOCK_NETWORK_MAX_BITS} bits",
            BlockStateId::COUNT
        );
    }
}
