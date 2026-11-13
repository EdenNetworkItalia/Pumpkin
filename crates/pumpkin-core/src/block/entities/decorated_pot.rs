use super::BlockEntity;
use pumpkin_data::data_component_impl::{DataComponentImpl, PotDecorationsImpl};
use pumpkin_data::item_stack::ItemStack;
use pumpkin_nbt::compound::NbtCompound;
use pumpkin_nbt::tag::NbtTag;
use pumpkin_util::math::position::BlockPos;
use std::sync::Mutex;

pub struct DecoratedPotBlockEntity {
    pub position: BlockPos,
    pub sherds: Mutex<PotDecorationsImpl>,
    pub item: Mutex<Option<ItemStack>>,
}

impl BlockEntity for DecoratedPotBlockEntity {
    fn resource_location(&self) -> &'static str {
        Self::ID
    }

    fn get_position(&self) -> BlockPos {
        self.position
    }

    fn from_nbt(nbt: &pumpkin_nbt::compound::NbtCompound, position: BlockPos) -> Self
    where
        Self: Sized,
    {
        let sherds = nbt
            .get("sherds")
            .and_then(PotDecorationsImpl::read_data)
            .unwrap_or_default();
        let item = nbt
            .get_compound("item")
            .and_then(ItemStack::read_item_stack);
        Self {
            position,
            sherds: Mutex::new(sherds),
            item: Mutex::new(item),
        }
    }

    fn write_nbt(&self, nbt: &mut NbtCompound) {
        if let Ok(sherds) = self.sherds.lock()
            && !sherds.is_empty()
        {
            nbt.put("sherds", sherds.write_data());
        }
        if let Ok(item) = self.item.lock()
            && let Some(it) = item.as_ref()
        {
            let mut it_nbt = NbtCompound::new();
            it.write_item_stack(&mut it_nbt);
            nbt.put_compound("item", it_nbt);
        }
    }

    fn chunk_data_nbt(&self) -> Option<NbtCompound> {
        let mut nbt = NbtCompound::new();
        if let Ok(sherds) = self.sherds.try_lock()
            && !sherds.is_empty()
        {
            nbt.put("sherds", sherds.write_data());
        }
        if let Ok(item) = self.item.try_lock()
            && let Some(ref it) = *item
        {
            let mut it_nbt = NbtCompound::new();
            it.write_item_stack(&mut it_nbt);
            nbt.put_compound("item", it_nbt);
        }
        Some(nbt)
    }

    fn bedrock_block_actor_data(
        &self,
        _state_id: pumpkin_data::BlockStateId,
    ) -> Option<NbtCompound> {
        let sherds = self
            .sherds
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Some(build_decorated_pot_actor(self.position, &sherds))
    }

    fn collect_item_components(&self, stack: &mut ItemStack) {
        if let Ok(sherds) = self.sherds.lock()
            && !sherds.is_empty()
        {
            stack.set_data_component(sherds.clone());
        }
        if let Ok(item) = self.item.lock()
            && let Some(it) = item.as_ref()
            && !it.is_empty()
        {
            stack.set_data_component(pumpkin_data::data_component_impl::ContainerImpl {
                items: vec![(0, it.clone())],
            });
        }
    }

    fn apply_item_components(&self, stack: &ItemStack) {
        if let Some(decorations) = stack.get_data_component::<PotDecorationsImpl>()
            && !decorations.is_empty()
        {
            *self
                .sherds
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = decorations.clone();
        }
        if let Some(container) =
            stack.get_data_component::<pumpkin_data::data_component_impl::ContainerImpl>()
            && let Some((_, it)) = container.items.first()
            && !it.is_empty()
        {
            *self
                .item
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(it.clone());
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl DecoratedPotBlockEntity {
    pub const ID: &'static str = "minecraft:decorated_pot";

    #[must_use]
    pub const fn new(position: BlockPos) -> Self {
        Self {
            position,
            sherds: Mutex::new(PotDecorationsImpl),
            item: Mutex::new(None),
        }
    }

    pub fn get_item(&self) -> Option<ItemStack> {
        self.item
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    pub fn take_item(&self) -> Option<ItemStack> {
        self.item
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
    }

    pub fn try_insert_item(&self, stack: &mut ItemStack, count: u8) -> bool {
        let mut item_guard = self
            .item
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(existing) = item_guard.as_mut() {
            if existing.item.id == stack.item.id {
                // Vanilla gates on the item's own max stack size, not a fixed 64.
                // Saturate so a count loaded from NBT above that limit cannot underflow.
                let space = existing
                    .get_max_stack_size()
                    .saturating_sub(existing.item_count);
                let add = count.min(space).min(stack.item_count);
                if add > 0 {
                    existing.item_count += add;
                    stack.item_count -= add;
                    return true;
                }
            }
            false
        } else {
            let insert_count = count.min(stack.item_count).min(stack.get_max_stack_size());
            let mut inserted = stack.clone();
            inserted.item_count = insert_count;
            *item_guard = Some(inserted);
            stack.item_count -= insert_count;
            true
        }
    }

    pub fn get_comparator_output(&self) -> u8 {
        self.item
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
            .map_or(0, |item| {
                if item.item_count == 0 {
                    0
                } else {
                    // Vanilla scales by the item's own stack size, not a fixed 64.
                    // `try_insert_item` may hold more than one stack, so cap the ratio if full.
                    let max_count = f32::from(item.get_max_stack_size());
                    let filled = (f32::from(item.item_count) / max_count).min(1.0);
                    1 + (filled * 14.0).floor() as u8
                }
            })
    }
}

/// Builds the Bedrock block-actor NBT for decorated pots: four item names, an empty side as a
/// brick.
#[must_use]
pub fn build_decorated_pot_actor(position: BlockPos, sherds: &PotDecorationsImpl) -> NbtCompound {
    let sherd_tags = sherds
        .ordered()
        .into_iter()
        .map(|id| NbtTag::String(id.into_owned().into()))
        .collect();

    super::bedrock::BedrockBlockActorBuilder::new("DecoratedPot", position)
        .put_list("sherds", sherd_tags)
        .finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decorated_pot_actor_builder() {
        let pos = BlockPos::new(2, 64, 2);
        let sherds = PotDecorationsImpl::from_list([
            Some("minecraft:archer_pottery_sherd"),
            Some("minecraft:brick"),
            Some("minecraft:prize_pottery_sherd"),
            Some("minecraft:miner_pottery_sherd"),
        ]);
        let nbt = build_decorated_pot_actor(pos, &sherds);
        assert_eq!(nbt.get_string("id"), Some("DecoratedPot"));
        let list = nbt.get_list("sherds").expect("sherds missing");
        assert_eq!(list.len(), 4);
        assert_eq!(
            list[0].extract_string(),
            Some("minecraft:archer_pottery_sherd")
        );
        assert_eq!(list[1].extract_string(), Some("minecraft:brick"));

        // An empty side is a brick on Bedrock too.
        let mut front_only = PotDecorationsImpl::default();
        front_only.sherds[3] = Some("minecraft:heart_pottery_sherd".into());
        let nbt = build_decorated_pot_actor(pos, &front_only);
        let list = nbt.get_list("sherds").expect("sherds missing");
        let names: Vec<_> = list.iter().filter_map(NbtTag::extract_string).collect();
        assert_eq!(
            names,
            [
                "minecraft:brick",
                "minecraft:brick",
                "minecraft:brick",
                "minecraft:heart_pottery_sherd"
            ]
        );
    }

    fn sherd_ids(sherds: &NbtTag) -> [Option<&str>; 4] {
        let NbtTag::Compound(map) = sherds else {
            panic!("sherds must be written as a compound");
        };
        ["back", "left", "right", "front"].map(|side| {
            map.get_compound(side)
                .and_then(|side| side.get_string("id"))
        })
    }

    #[test]
    fn sherds_are_written_as_the_four_side_map_and_read_from_both_shapes() {
        // 26.3 `DecoratedPotBlockEntity` stores `sherds` with `PotDecorations.CODEC`.
        let expected = [
            Some("minecraft:archer_pottery_sherd"),
            Some("minecraft:brick"),
            Some("minecraft:prize_pottery_sherd"),
            Some("minecraft:miner_pottery_sherd"),
        ];
        let ids = expected.map(|id| NbtTag::String(id.unwrap().into()));

        let mut as_list = NbtCompound::new();
        as_list.put_list("sherds", ids.to_vec());
        let pot = DecoratedPotBlockEntity::from_nbt(&as_list, BlockPos::new(0, 0, 0));

        let mut written = NbtCompound::new();
        pot.write_nbt(&mut written);
        assert_eq!(sherd_ids(written.get("sherds").unwrap()), expected);
        assert_eq!(
            sherd_ids(pot.chunk_data_nbt().unwrap().get("sherds").unwrap()),
            expected
        );

        let reloaded = DecoratedPotBlockEntity::from_nbt(&written, BlockPos::new(0, 0, 0));
        let mut rewritten = NbtCompound::new();
        reloaded.write_nbt(&mut rewritten);
        assert_eq!(sherd_ids(rewritten.get("sherds").unwrap()), expected);
    }

    #[test]
    fn a_missing_side_stays_empty_and_no_sherds_write_nothing() {
        // `PotDecorations.CODEC` reads every side with `optionalFieldOf`.
        let mut front = NbtCompound::new();
        front.put_string("id", "minecraft:heart_pottery_sherd".to_string());
        let mut map = NbtCompound::new();
        map.put_compound("front", front);
        let mut nbt = NbtCompound::new();
        nbt.put_compound("sherds", map);
        let pot = DecoratedPotBlockEntity::from_nbt(&nbt, BlockPos::new(0, 0, 0));

        let mut written = NbtCompound::new();
        pot.write_nbt(&mut written);
        assert_eq!(
            sherd_ids(written.get("sherds").unwrap()),
            [None, None, None, Some("minecraft:heart_pottery_sherd")]
        );

        // `DecoratedPotBlockEntity.saveAdditional` skips `PotDecorations.EMPTY`.
        let plain = DecoratedPotBlockEntity::from_nbt(&NbtCompound::new(), BlockPos::new(0, 0, 0));
        let mut written = NbtCompound::new();
        plain.write_nbt(&mut written);
        assert!(written.get("sherds").is_none());
    }
}
