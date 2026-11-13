use crate::data_component_impl::{
    DataComponentImpl, IdOr, get_i32_hash, get_idor_hash, get_str_hash,
};
use crate::sound::Sound;
use pumpkin_nbt::compound::NbtCompound;
use pumpkin_nbt::tag::NbtTag;
use pumpkin_util::text::TextComponent;
use std::borrow::Cow;

#[derive(Clone, Debug, PartialEq)]
pub struct CustomDataImpl {
    pub data: NbtCompound,
}
impl CustomDataImpl {
    #[must_use]
    pub const fn new(data: NbtCompound) -> Self {
        Self { data }
    }
    #[must_use]
    pub fn read_data(tag: &NbtTag) -> Option<Self> {
        if let NbtTag::Compound(c) = tag {
            Some(Self { data: c.clone() })
        } else {
            None
        }
    }
}
impl DataComponentImpl for CustomDataImpl {
    fn write_data(&self) -> NbtTag {
        NbtTag::Compound(self.data.clone())
    }
    fn get_hash(&self) -> i32 {
        0
    }
    default_impl!(CustomData);
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct MaxStackSizeImpl {
    pub size: u8,
}
impl MaxStackSizeImpl {
    pub fn read_data(data: &NbtTag) -> Option<Self> {
        data.extract_int().map(|size| Self { size: size as u8 })
    }
}
impl DataComponentImpl for MaxStackSizeImpl {
    fn write_data(&self) -> NbtTag {
        NbtTag::Int(self.size as i32)
    }
    fn get_hash(&self) -> i32 {
        get_i32_hash(self.size as i32) as i32
    }
    default_impl!(MaxStackSize);
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct MaxDamageImpl {
    pub max_damage: i32,
}
impl MaxDamageImpl {
    pub fn read_data(data: &NbtTag) -> Option<Self> {
        data.extract_int().map(|max_damage| Self { max_damage })
    }
}
impl DataComponentImpl for MaxDamageImpl {
    fn write_data(&self) -> NbtTag {
        NbtTag::Int(self.max_damage)
    }
    default_impl!(MaxDamage);
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct DamageImpl {
    pub damage: i32,
}
impl DamageImpl {
    pub fn read_data(data: &NbtTag) -> Option<Self> {
        data.extract_int().map(|damage| Self { damage })
    }
}
impl DataComponentImpl for DamageImpl {
    fn write_data(&self) -> NbtTag {
        NbtTag::Int(self.damage)
    }
    fn get_hash(&self) -> i32 {
        get_i32_hash(self.damage) as i32
    }
    default_impl!(Damage);
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct UnbreakableImpl;
impl UnbreakableImpl {
    pub const fn read_data(_data: &NbtTag) -> Option<Self> {
        Some(Self)
    }
}
impl DataComponentImpl for UnbreakableImpl {
    fn write_data(&self) -> NbtTag {
        NbtTag::Compound(NbtCompound::new())
    }
    fn get_hash(&self) -> i32 {
        0
    }
    default_impl!(Unbreakable);
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct CustomNameImpl {
    pub name: TextComponent,
}
impl CustomNameImpl {
    pub fn read_data(data: &NbtTag) -> Option<Self> {
        data.extract_string().map(|name| Self {
            name: TextComponent::text(name.to_string()),
        })
    }
}
impl DataComponentImpl for CustomNameImpl {
    fn write_data(&self) -> NbtTag {
        NbtTag::String(self.name.clone().get_text().into())
    }
    fn get_hash(&self) -> i32 {
        get_str_hash(self.name.clone().get_text().as_str()) as i32
    }
    default_impl!(CustomName);
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct ItemNameImpl {
    pub name: Cow<'static, str>,
}
impl ItemNameImpl {
    pub fn read_data(data: &NbtTag) -> Option<Self> {
        let name = match data {
            NbtTag::String(name) => name.to_string(),
            NbtTag::Compound(component) => component
                .get_string("translate")
                .or_else(|| component.get_string("text"))?
                .to_owned(),
            _ => return None,
        };
        Some(Self {
            name: Cow::Owned(name),
        })
    }
}
impl DataComponentImpl for ItemNameImpl {
    fn write_data(&self) -> NbtTag {
        let mut component = NbtCompound::new();
        component.put_string("translate", self.name.to_string());
        NbtTag::Compound(component)
    }
    fn get_hash(&self) -> i32 {
        get_str_hash(&self.name) as i32
    }
    default_impl!(ItemName);
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct ItemModelImpl {
    pub id: Cow<'static, str>,
}
impl ItemModelImpl {
    pub fn read_data(data: &NbtTag) -> Option<Self> {
        data.extract_string().map(|id| Self {
            id: Cow::Owned(id.to_string()),
        })
    }
}
impl DataComponentImpl for ItemModelImpl {
    fn write_data(&self) -> NbtTag {
        NbtTag::String(self.id.clone().into_owned().into())
    }
    fn get_hash(&self) -> i32 {
        get_str_hash(self.id.as_ref()) as i32
    }
    default_impl!(ItemModel);
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct LoreImpl {
    pub lines: Vec<TextComponent>,
}
impl LoreImpl {
    pub fn read_data(data: &NbtTag) -> Option<Self> {
        let NbtTag::List(lines) = data else {
            return None;
        };

        Some(Self {
            lines: lines
                .iter()
                .filter_map(NbtTag::extract_string)
                .map(|line| TextComponent::text(line.to_owned()))
                .collect(),
        })
    }
}
impl DataComponentImpl for LoreImpl {
    fn write_data(&self) -> NbtTag {
        NbtTag::List(
            self.lines
                .iter()
                .map(|line| NbtTag::String(line.clone().get_text().into_boxed_str()))
                .collect(),
        )
    }
    default_impl!(Lore);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Rarity {
    #[default]
    Common = 0,
    Uncommon = 1,
    Rare = 2,
    Epic = 3,
}

impl Rarity {
    #[must_use]
    pub fn from_id(id: i32) -> Option<Self> {
        match id {
            0 => Some(Self::Common),
            1 => Some(Self::Uncommon),
            2 => Some(Self::Rare),
            3 => Some(Self::Epic),
            _ => None,
        }
    }

    #[must_use]
    pub fn to_id(self) -> i32 {
        self as i32
    }

    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "common" => Some(Self::Common),
            "uncommon" => Some(Self::Uncommon),
            "rare" => Some(Self::Rare),
            "epic" => Some(Self::Epic),
            _ => None,
        }
    }

    #[must_use]
    pub fn to_name(self) -> &'static str {
        match self {
            Self::Common => "common",
            Self::Uncommon => "uncommon",
            Self::Rare => "rare",
            Self::Epic => "epic",
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct RarityImpl {
    pub rarity: Rarity,
}

impl RarityImpl {
    pub fn read_data(data: &NbtTag) -> Option<Self> {
        let name = data.extract_string()?;
        Some(Self {
            rarity: Rarity::from_name(name)?,
        })
    }
}

impl DataComponentImpl for RarityImpl {
    fn write_data(&self) -> NbtTag {
        NbtTag::String(self.rarity.to_name().into())
    }

    fn get_hash(&self) -> i32 {
        crate::data_component_impl::get_i32_hash(self.rarity.to_id()) as i32
    }

    default_impl!(Rarity);
}

#[derive(Clone, Debug, PartialEq)]
pub struct CustomModelDataImpl {
    pub floats: Vec<f32>,
    pub flags: Vec<bool>,
    pub strings: Vec<String>,
    pub colors: Vec<i32>,
}
impl CustomModelDataImpl {
    pub fn read_data(data: &NbtTag) -> Option<Self> {
        let compound = data.extract_compound()?;
        let floats = compound
            .get_list("floats")
            .map(|l| l.iter().filter_map(NbtTag::extract_float).collect())
            .unwrap_or_default();
        let flags = compound
            .get_list("flags")
            .map(|l| l.iter().filter_map(NbtTag::extract_bool).collect())
            .unwrap_or_default();
        let strings = compound
            .get_list("strings")
            .map(|l| {
                l.iter()
                    .filter_map(|t| t.extract_string().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        // Vanilla encodes the color list as ints, but tolerate a packed int array too.
        let colors = if let Some(arr) = compound.get_int_array("colors") {
            arr.to_vec()
        } else if let Some(l) = compound.get_list("colors") {
            l.iter().filter_map(NbtTag::extract_int).collect()
        } else {
            Vec::new()
        };
        Some(Self {
            floats,
            flags,
            strings,
            colors,
        })
    }
}
impl DataComponentImpl for CustomModelDataImpl {
    fn write_data(&self) -> NbtTag {
        let mut compound = NbtCompound::new();
        compound.put_list(
            "floats",
            self.floats.iter().map(|f| NbtTag::Float(*f)).collect(),
        );
        compound.put_list(
            "flags",
            self.flags.iter().map(|b| NbtTag::Byte(*b as i8)).collect(),
        );
        compound.put_list(
            "strings",
            self.strings
                .iter()
                .map(|s| NbtTag::String(s.clone().into()))
                .collect(),
        );
        compound.put_list(
            "colors",
            self.colors.iter().map(|c| NbtTag::Int(*c)).collect(),
        );
        NbtTag::Compound(compound)
    }
    default_impl!(CustomModelData);
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct TooltipDisplayImpl;
impl TooltipDisplayImpl {
    pub const fn read_data(_data: &NbtTag) -> Option<Self> {
        Some(Self)
    }
}
impl DataComponentImpl for TooltipDisplayImpl {
    default_impl!(TooltipDisplay);
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct CreativeSlotLockImpl;
impl CreativeSlotLockImpl {
    pub const fn read_data(_data: &NbtTag) -> Option<Self> {
        Some(Self)
    }
}
impl DataComponentImpl for CreativeSlotLockImpl {
    default_impl!(CreativeSlotLock);
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct EnchantmentGlintOverrideImpl {
    pub glint: bool,
}
impl EnchantmentGlintOverrideImpl {
    pub fn read_data(data: &NbtTag) -> Option<Self> {
        Some(Self {
            glint: data.extract_bool().unwrap_or(true),
        })
    }
}
impl DataComponentImpl for EnchantmentGlintOverrideImpl {
    fn write_data(&self) -> NbtTag {
        NbtTag::Byte(self.glint as i8)
    }
    default_impl!(EnchantmentGlintOverride);
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct TooltipStyleImpl {
    pub id: String,
}
impl TooltipStyleImpl {
    pub fn read_data(data: &NbtTag) -> Option<Self> {
        data.extract_string().map(|id| Self { id: id.to_string() })
    }
}
impl DataComponentImpl for TooltipStyleImpl {
    fn write_data(&self) -> NbtTag {
        NbtTag::String(self.id.clone().into())
    }
    fn get_hash(&self) -> i32 {
        get_str_hash(&self.id) as i32
    }
    default_impl!(TooltipStyle);
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct NoteBlockSoundImpl {
    pub sound: String,
}
impl NoteBlockSoundImpl {
    pub fn read_data(data: &NbtTag) -> Option<Self> {
        data.extract_string().map(|sound| Self {
            sound: sound.to_string(),
        })
    }
}
impl DataComponentImpl for NoteBlockSoundImpl {
    fn write_data(&self) -> NbtTag {
        NbtTag::String(self.sound.clone().into())
    }
    fn get_hash(&self) -> i32 {
        get_str_hash(&self.sound) as i32
    }
    default_impl!(NoteBlockSound);
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct BaseColorImpl {
    pub color: String,
}
impl BaseColorImpl {
    pub fn read_data(data: &NbtTag) -> Option<Self> {
        data.extract_string().map(|color| Self {
            color: color.to_string(),
        })
    }
}
impl DataComponentImpl for BaseColorImpl {
    fn write_data(&self) -> NbtTag {
        NbtTag::String(self.color.clone().into())
    }
    fn get_hash(&self) -> i32 {
        get_str_hash(&self.color) as i32
    }
    default_impl!(BaseColor);
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct InstrumentImpl {
    pub instrument: Cow<'static, str>,
}
impl InstrumentImpl {
    pub fn read_data(data: &NbtTag) -> Option<Self> {
        let instrument = match data {
            NbtTag::String(s) => s.to_string(),
            NbtTag::Compound(c) => c.get_string("instrument")?.to_string(),
            _ => return None,
        };
        Some(Self {
            instrument: Cow::Owned(instrument),
        })
    }
}
impl DataComponentImpl for InstrumentImpl {
    fn write_data(&self) -> NbtTag {
        NbtTag::String(self.instrument.to_string().into())
    }
    fn get_hash(&self) -> i32 {
        get_str_hash(&self.instrument) as i32
    }
    default_impl!(Instrument);
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct ProvidesTrimMaterialImpl {
    pub material: Cow<'static, str>,
}
impl ProvidesTrimMaterialImpl {
    pub fn read_data(data: &NbtTag) -> Option<Self> {
        data.extract_string().map(|material| Self {
            material: Cow::Owned(material.to_string()),
        })
    }
}
impl DataComponentImpl for ProvidesTrimMaterialImpl {
    fn write_data(&self) -> NbtTag {
        NbtTag::String(self.material.to_string().into())
    }
    fn get_hash(&self) -> i32 {
        get_str_hash(&self.material) as i32
    }
    default_impl!(ProvidesTrimMaterial);
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct ProvidesBannerPatternsImpl {
    pub key: Cow<'static, str>,
}
impl ProvidesBannerPatternsImpl {
    pub fn read_data(data: &NbtTag) -> Option<Self> {
        data.extract_string().map(|key| Self {
            key: Cow::Owned(key.to_string()),
        })
    }
}
impl DataComponentImpl for ProvidesBannerPatternsImpl {
    fn write_data(&self) -> NbtTag {
        NbtTag::String(self.key.to_string().into())
    }
    fn get_hash(&self) -> i32 {
        get_str_hash(&self.key) as i32
    }
    default_impl!(ProvidesBannerPatterns);
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct ProvidesPotteryPatternImpl {
    pub pattern: Cow<'static, str>,
}
impl ProvidesPotteryPatternImpl {
    pub fn read_data(data: &NbtTag) -> Option<Self> {
        data.extract_string().map(|pattern| Self {
            pattern: Cow::Owned(pattern.to_string()),
        })
    }
}
impl DataComponentImpl for ProvidesPotteryPatternImpl {
    fn write_data(&self) -> NbtTag {
        NbtTag::String(self.pattern.to_string().into())
    }
    fn get_hash(&self) -> i32 {
        get_str_hash(&self.pattern) as i32
    }
    default_impl!(ProvidesPotteryPattern);
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct CushionColorImpl {
    pub color: crate::dye_color::DyeColor,
}
impl CushionColorImpl {
    pub fn read_data(data: &NbtTag) -> Option<Self> {
        let color = if let Some(s) = data.extract_string() {
            crate::dye_color::DyeColor::by_name(s)?
        } else if let Some(id) = data.extract_int() {
            crate::dye_color::DyeColor::by_id(id as u8)?
        } else {
            return None;
        };
        Some(Self { color })
    }
}
impl DataComponentImpl for CushionColorImpl {
    fn write_data(&self) -> NbtTag {
        NbtTag::String(self.color.name().into())
    }
    fn get_hash(&self) -> i32 {
        get_str_hash(self.color.name()) as i32
    }
    default_impl!(CushionColor);
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct BannerPatternLayer {
    pub pattern: String,
    pub color: crate::dye_color::DyeColor,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Default)]
pub struct BannerPatternsImpl {
    pub layers: Vec<BannerPatternLayer>,
}

impl BannerPatternsImpl {
    pub const EMPTY: Self = Self { layers: Vec::new() };

    #[must_use]
    pub fn read_data(data: &NbtTag) -> Option<Self> {
        let mut layers = Vec::new();
        if let NbtTag::List(list) = data {
            for tag in list {
                if let Some(compound) = tag.extract_compound() {
                    let pattern = compound.get_string("pattern")?.to_string();
                    let color_str = compound.get_string("color")?;
                    let color = crate::dye_color::DyeColor::by_name(color_str).unwrap_or_default();
                    layers.push(BannerPatternLayer { pattern, color });
                }
            }
        }
        Some(Self { layers })
    }
}

impl DataComponentImpl for BannerPatternsImpl {
    fn write_data(&self) -> NbtTag {
        let mut list = Vec::new();
        for layer in &self.layers {
            let mut compound = NbtCompound::new();
            compound.put_string("pattern", layer.pattern.clone());
            compound.put_string("color", layer.color.name().to_string());
            list.push(NbtTag::Compound(compound));
        }
        NbtTag::List(list)
    }

    default_impl!(BannerPatterns);
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Default)]
pub struct PotDecorationsImpl {
    /// The sides in `PotDecorations.CODEC` order: back, left, right, front. `None` is an empty
    /// side, which vanilla draws like a brick and does not drop.
    pub sherds: [Option<Cow<'static, str>>; 4],
}
#[allow(non_upper_case_globals)]
pub const PotDecorationsImpl: PotDecorationsImpl = PotDecorationsImpl {
    sherds: [None, None, None, None],
};
/// The sides of a decorated pot in `PotDecorations.CODEC` order, which is also the order of the
/// 26.2 list.
const POT_DECORATION_SIDES: [&str; 4] = ["back", "left", "right", "front"];
const POT_DECORATION_BRICK: &str = "minecraft:brick";

impl PotDecorationsImpl {
    /// The sides of a 26.2 list of item ids, as `PotDecorationsComponentUnflatteningFix` reads
    /// it: a missing or unreadable entry is a brick, so every side is present.
    pub fn from_list<'a>(ids: impl IntoIterator<Item = Option<&'a str>>) -> Self {
        let mut ids = ids.into_iter();
        Self {
            sherds: std::array::from_fn(|_| {
                Some(Cow::Owned(
                    ids.next()
                        .flatten()
                        .unwrap_or(POT_DECORATION_BRICK)
                        .to_string(),
                ))
            }),
        }
    }

    /// Whether every side is empty, `PotDecorations.EMPTY`.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sherds.iter().all(Option::is_none)
    }

    /// The four sides with an empty side as a brick, in `PotDecorations.CODEC` order.
    #[must_use]
    pub fn ordered(&self) -> [Cow<'static, str>; 4] {
        self.sherds
            .clone()
            .map(|id| id.unwrap_or(Cow::Borrowed(POT_DECORATION_BRICK)))
    }

    /// Reads the 26.3 map of four optional `{id}` sides, or the 26.2 list of item ids.
    pub fn read_data(data: &NbtTag) -> Option<Self> {
        match data {
            NbtTag::List(list) => Some(Self::from_list(list.iter().map(NbtTag::extract_string))),
            // `ItemStackTemplate.CODEC` accepts a compound with `id` or a bare item id.
            NbtTag::Compound(map) => Some(Self {
                sherds: POT_DECORATION_SIDES.map(|side| {
                    match map.get(side)? {
                        NbtTag::Compound(stack) => stack.get_string("id"),
                        tag => tag.extract_string(),
                    }
                    .map(|id| Cow::Owned(id.to_string()))
                }),
            }),
            _ => Some(Self::default()),
        }
    }
}
impl DataComponentImpl for PotDecorationsImpl {
    /// Writes `PotDecorations.CODEC`: the present sides only.
    fn write_data(&self) -> NbtTag {
        let mut map = NbtCompound::new();
        for (side, id) in POT_DECORATION_SIDES.into_iter().zip(&self.sherds) {
            if let Some(id) = id {
                let mut stack = NbtCompound::new();
                stack.put_string("id", id.to_string());
                map.put_compound(side, stack);
            }
        }
        NbtTag::Compound(map)
    }
    default_impl!(PotDecorations);
}

/// The lock's item predicate, kept as its raw NBT compound since Pumpkin does
/// not yet model item predicates.
// TODO: replace `predicate` with a typed item predicate once item predicates are modelled.
#[derive(Clone, Debug, PartialEq)]
pub struct LockImpl {
    pub predicate: NbtCompound,
}
impl LockImpl {
    pub fn read_data(data: &NbtTag) -> Option<Self> {
        data.extract_compound().map(|predicate| Self {
            predicate: predicate.clone(),
        })
    }
}
impl DataComponentImpl for LockImpl {
    fn write_data(&self) -> NbtTag {
        NbtTag::Compound(self.predicate.clone())
    }
    default_impl!(Lock);
}

#[derive(Clone, Debug, Hash, PartialEq)]
pub struct BreakSoundImpl {
    pub sound: IdOr<SoundEvent>,
}
impl BreakSoundImpl {
    pub fn read_data(data: &NbtTag) -> Option<Self> {
        let sound = if let Some(s) = data.extract_string() {
            let name = s.strip_prefix("minecraft:").unwrap_or(s);
            if let Some(sound) = Sound::from_name(name) {
                IdOr::Id(sound)
            } else {
                IdOr::Value(SoundEvent {
                    sound_name: s.to_string(),
                    range: None,
                })
            }
        } else if let Some(c) = data.extract_compound() {
            let sound_name = c.get_string("sound_id")?.to_string();
            let range = c.get_float("range");
            IdOr::Value(SoundEvent { sound_name, range })
        } else {
            return None;
        };
        Some(Self { sound })
    }
}
impl DataComponentImpl for BreakSoundImpl {
    fn write_data(&self) -> NbtTag {
        match &self.sound {
            IdOr::Id(sound) => NbtTag::String(format!("minecraft:{}", sound.to_name()).into()),
            IdOr::Value(event) => {
                let mut compound = NbtCompound::new();
                compound.put_string("sound_id", event.sound_name.clone());
                if let Some(range) = event.range {
                    compound.put_float("range", range);
                }
                NbtTag::Compound(compound)
            }
        }
    }
    fn get_hash(&self) -> i32 {
        get_idor_hash(&self.sound) as i32
    }
    default_impl!(BreakSound);
}

#[derive(Clone, Debug, PartialEq)]
pub struct SoundEvent {
    pub sound_name: String,
    pub range: Option<f32>,
}
impl std::hash::Hash for SoundEvent {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.sound_name.hash(state);
        if let Some(val) = self.range {
            true.hash(state);
            unsafe { (*(&raw const val).cast::<u32>()).hash(state) };
        } else {
            false.hash(state);
        }
    }
}
impl SoundEvent {
    pub const fn new(sound_name: String, range: Option<f32>) -> Self {
        Self { sound_name, range }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decorations() -> PotDecorationsImpl {
        PotDecorationsImpl {
            sherds: [
                Some(Cow::Borrowed("minecraft:archer_pottery_sherd")),
                Some(Cow::Borrowed("minecraft:brick")),
                Some(Cow::Borrowed("minecraft:prize_pottery_sherd")),
                Some(Cow::Borrowed("minecraft:miner_pottery_sherd")),
            ],
        }
    }

    fn side_id<'a>(map: &'a NbtCompound, side: &str) -> Option<&'a str> {
        map.get_compound(side)?.get_string("id")
    }

    #[test]
    fn pot_decorations_write_the_four_side_map() {
        // 26.3 `PotDecorations.CODEC`: a record of `back`, `left`, `right` and `front`, each an
        // item stack template with an `id`.
        let NbtTag::Compound(map) = decorations().write_data() else {
            panic!("pot decorations must be written as a compound");
        };
        assert_eq!(
            side_id(&map, "back"),
            Some("minecraft:archer_pottery_sherd")
        );
        assert_eq!(side_id(&map, "left"), Some("minecraft:brick"));
        assert_eq!(
            side_id(&map, "right"),
            Some("minecraft:prize_pottery_sherd")
        );
        assert_eq!(
            side_id(&map, "front"),
            Some("minecraft:miner_pottery_sherd")
        );
    }

    #[test]
    fn pot_decorations_read_the_map_and_the_26_2_list() {
        let from_map = PotDecorationsImpl::read_data(&decorations().write_data()).unwrap();
        assert_eq!(from_map, decorations());

        // `PotDecorationsComponentUnflatteningFix`: list index 0 to 3 is back, left, right, front.
        let list = NbtTag::List(
            decorations()
                .ordered()
                .iter()
                .map(|id| NbtTag::String(id.to_string().into()))
                .collect(),
        );
        assert_eq!(PotDecorationsImpl::read_data(&list).unwrap(), decorations());

        // The fix pads a short list with bricks.
        let short = NbtTag::List(vec![NbtTag::String(
            "minecraft:archer_pottery_sherd".into(),
        )]);
        assert_eq!(
            PotDecorationsImpl::read_data(&short).unwrap().sherds,
            [
                Some(Cow::Borrowed("minecraft:archer_pottery_sherd")),
                Some(Cow::Borrowed("minecraft:brick")),
                Some(Cow::Borrowed("minecraft:brick")),
                Some(Cow::Borrowed("minecraft:brick")),
            ]
        );
    }

    #[test]
    fn pot_decorations_map_reads_bare_ids_and_missing_sides() {
        // `ItemStackTemplate.CODEC` also accepts a bare item id, and every side of
        // `PotDecorations.CODEC` is optional: a missing side stays empty.
        let mut map = NbtCompound::new();
        map.put_string("left", "minecraft:archer_pottery_sherd".to_string());
        let mut front = NbtCompound::new();
        front.put_string("id", "minecraft:miner_pottery_sherd".to_string());
        map.put_compound("front", front);

        let read = PotDecorationsImpl::read_data(&NbtTag::Compound(map)).unwrap();
        assert_eq!(
            read.sherds,
            [
                None,
                Some(Cow::Borrowed("minecraft:archer_pottery_sherd")),
                None,
                Some(Cow::Borrowed("minecraft:miner_pottery_sherd")),
            ]
        );
        assert_eq!(
            read.ordered(),
            [
                "minecraft:brick",
                "minecraft:archer_pottery_sherd",
                "minecraft:brick",
                "minecraft:miner_pottery_sherd",
            ]
        );

        // Only the present sides are written back.
        let NbtTag::Compound(written) = read.write_data() else {
            panic!("pot decorations must be written as a compound");
        };
        assert_eq!(written.child_tags.len(), 2);
        assert_eq!(
            side_id(&written, "left"),
            Some("minecraft:archer_pottery_sherd")
        );
        assert_eq!(
            side_id(&written, "front"),
            Some("minecraft:miner_pottery_sherd")
        );
        assert!(
            PotDecorationsImpl::read_data(&NbtTag::Compound(NbtCompound::new()))
                .unwrap()
                .is_empty()
        );
    }
}
