#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StructurePieceType {
    // Mineshaft
    MineshaftCorridor,
    MineshaftCrossing,
    MineshaftRoom,
    MineshaftStairs,

    // Nether Fortress
    NetherFortressBridgeCrossing,
    NetherFortressBridgeEnd,
    NetherFortressBridge,
    NetherFortressCorridorStairs,
    NetherFortressCorridorBalcony,
    NetherFortressCorridorExit,
    NetherFortressCorridorCrossing,
    NetherFortressCorridorLeftTurn,
    NetherFortressSmallCorridor,
    NetherFortressCorridorRightTurn,
    NetherFortressCorridorNetherWartsRoom,
    NetherFortressBridgePlatform,
    NetherFortressBridgeSmallCrossing,
    NetherFortressBridgeStairs,
    NetherFortressStart,

    // Stronghold
    StrongholdChestCorridor,
    StrongholdSmallCorridor,
    StrongholdFiveWayCrossing,
    StrongholdLeftTurn,
    StrongholdLibrary,
    StrongholdPortalRoom,
    StrongholdPrisonHall,
    StrongholdRightTurn,
    StrongholdSquareRoom,
    StrongholdSpiralStaircase,
    StrongholdStart,
    StrongholdCorridor,
    StrongholdStairs,

    // Overworld/General
    JungleTemple,
    OceanTemple,
    Igloo,
    RuinedPortal,
    SwampHut,
    DesertTemple,

    // Ocean Monument
    OceanMonumentBase,
    OceanMonumentCoreRoom,
    OceanMonumentDoubleXRoom,
    OceanMonumentDoubleXYRoom,
    OceanMonumentDoubleYRoom,
    OceanMonumentDoubleYZRoom,
    OceanMonumentDoubleZRoom,
    OceanMonumentEntryRoom,
    OceanMonumentPenthouse,
    OceanMonumentSimpleRoom,
    OceanMonumentSimpleTopRoom,
    OceanMonumentWingRoom,

    // End / Other
    EndCity,
    WoodlandMansion,
    BuriedTreasure,
    Shipwreck,
    NetherFossil,
    Jigsaw,
}

impl StructurePieceType {
    const ALL: [Self; 56] = [
        Self::MineshaftCorridor,
        Self::MineshaftCrossing,
        Self::MineshaftRoom,
        Self::MineshaftStairs,
        Self::NetherFortressBridgeCrossing,
        Self::NetherFortressBridgeEnd,
        Self::NetherFortressBridge,
        Self::NetherFortressCorridorStairs,
        Self::NetherFortressCorridorBalcony,
        Self::NetherFortressCorridorExit,
        Self::NetherFortressCorridorCrossing,
        Self::NetherFortressCorridorLeftTurn,
        Self::NetherFortressSmallCorridor,
        Self::NetherFortressCorridorRightTurn,
        Self::NetherFortressCorridorNetherWartsRoom,
        Self::NetherFortressBridgePlatform,
        Self::NetherFortressBridgeSmallCrossing,
        Self::NetherFortressBridgeStairs,
        Self::NetherFortressStart,
        Self::StrongholdChestCorridor,
        Self::StrongholdSmallCorridor,
        Self::StrongholdFiveWayCrossing,
        Self::StrongholdLeftTurn,
        Self::StrongholdLibrary,
        Self::StrongholdPortalRoom,
        Self::StrongholdPrisonHall,
        Self::StrongholdRightTurn,
        Self::StrongholdSquareRoom,
        Self::StrongholdSpiralStaircase,
        Self::StrongholdStart,
        Self::StrongholdCorridor,
        Self::StrongholdStairs,
        Self::JungleTemple,
        Self::OceanTemple,
        Self::Igloo,
        Self::RuinedPortal,
        Self::SwampHut,
        Self::DesertTemple,
        Self::OceanMonumentBase,
        Self::OceanMonumentCoreRoom,
        Self::OceanMonumentDoubleXRoom,
        Self::OceanMonumentDoubleXYRoom,
        Self::OceanMonumentDoubleYRoom,
        Self::OceanMonumentDoubleYZRoom,
        Self::OceanMonumentDoubleZRoom,
        Self::OceanMonumentEntryRoom,
        Self::OceanMonumentPenthouse,
        Self::OceanMonumentSimpleRoom,
        Self::OceanMonumentSimpleTopRoom,
        Self::OceanMonumentWingRoom,
        Self::EndCity,
        Self::WoodlandMansion,
        Self::BuriedTreasure,
        Self::Shipwreck,
        Self::NetherFossil,
        Self::Jigsaw,
    ];

    /// The key of this piece type in the vanilla `structure_piece` registry, which
    /// `StructurePiece.createTag` writes as `id`.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::MineshaftCorridor => "minecraft:mscorridor",
            Self::MineshaftCrossing => "minecraft:mscrossing",
            Self::MineshaftRoom => "minecraft:msroom",
            Self::MineshaftStairs => "minecraft:msstairs",
            Self::NetherFortressBridgeCrossing => "minecraft:nebcr",
            Self::NetherFortressBridgeEnd => "minecraft:nebef",
            Self::NetherFortressBridge => "minecraft:nebs",
            Self::NetherFortressCorridorStairs => "minecraft:neccs",
            Self::NetherFortressCorridorBalcony => "minecraft:nectb",
            Self::NetherFortressCorridorExit => "minecraft:nece",
            Self::NetherFortressCorridorCrossing => "minecraft:nescsc",
            Self::NetherFortressCorridorLeftTurn => "minecraft:nesclt",
            Self::NetherFortressSmallCorridor => "minecraft:nesc",
            Self::NetherFortressCorridorRightTurn => "minecraft:nescrt",
            Self::NetherFortressCorridorNetherWartsRoom => "minecraft:necsr",
            Self::NetherFortressBridgePlatform => "minecraft:nemt",
            Self::NetherFortressBridgeSmallCrossing => "minecraft:nerc",
            Self::NetherFortressBridgeStairs => "minecraft:nesr",
            Self::NetherFortressStart => "minecraft:nestart",
            Self::StrongholdChestCorridor => "minecraft:shcc",
            Self::StrongholdSmallCorridor => "minecraft:shfc",
            Self::StrongholdFiveWayCrossing => "minecraft:sh5c",
            Self::StrongholdLeftTurn => "minecraft:shlt",
            Self::StrongholdLibrary => "minecraft:shli",
            Self::StrongholdPortalRoom => "minecraft:shpr",
            Self::StrongholdPrisonHall => "minecraft:shph",
            Self::StrongholdRightTurn => "minecraft:shrt",
            Self::StrongholdSquareRoom => "minecraft:shrc",
            Self::StrongholdSpiralStaircase => "minecraft:shsd",
            Self::StrongholdStart => "minecraft:shstart",
            Self::StrongholdCorridor => "minecraft:shs",
            Self::StrongholdStairs => "minecraft:shssd",
            Self::JungleTemple => "minecraft:tejp",
            Self::OceanTemple => "minecraft:orp",
            Self::Igloo => "minecraft:iglu",
            Self::RuinedPortal => "minecraft:rupo",
            Self::SwampHut => "minecraft:tesh",
            Self::DesertTemple => "minecraft:tedp",
            Self::OceanMonumentBase => "minecraft:omb",
            Self::OceanMonumentCoreRoom => "minecraft:omcr",
            Self::OceanMonumentDoubleXRoom => "minecraft:omdxr",
            Self::OceanMonumentDoubleXYRoom => "minecraft:omdxyr",
            Self::OceanMonumentDoubleYRoom => "minecraft:omdyr",
            Self::OceanMonumentDoubleYZRoom => "minecraft:omdyzr",
            Self::OceanMonumentDoubleZRoom => "minecraft:omdzr",
            Self::OceanMonumentEntryRoom => "minecraft:omentry",
            Self::OceanMonumentPenthouse => "minecraft:ompenthouse",
            Self::OceanMonumentSimpleRoom => "minecraft:omsimple",
            Self::OceanMonumentSimpleTopRoom => "minecraft:omsimplet",
            Self::OceanMonumentWingRoom => "minecraft:omwr",
            Self::EndCity => "minecraft:ecp",
            Self::WoodlandMansion => "minecraft:wmp",
            Self::BuriedTreasure => "minecraft:btp",
            Self::Shipwreck => "minecraft:shipwreck",
            Self::NetherFossil => "minecraft:nefos",
            Self::Jigsaw => "minecraft:jigsaw",
        }
    }

    /// The piece type of a saved piece `id`, as vanilla `PiecesContainer.load` reads it: in
    /// lower case, in the `minecraft` namespace when the id has none, and with the jigsaw ids of
    /// old versions renamed.
    #[must_use]
    pub fn from_id(id: &str) -> Option<Self> {
        let id = id.to_ascii_lowercase();
        let path = match super::start::minecraft_path(&id)? {
            "nvi" | "pcp" | "bastionremnant" | "runtime" => "jigsaw",
            path => path,
        };
        Self::ALL
            .into_iter()
            .find(|piece_type| piece_type.id().strip_prefix("minecraft:") == Some(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Vanilla `PiecesContainer.load` reads piece ids in lower case, in the default namespace,
    /// and with the jigsaw ids of old versions renamed.
    #[test]
    fn piece_ids_read_like_vanilla() {
        for piece_type in StructurePieceType::ALL {
            assert_eq!(
                StructurePieceType::from_id(piece_type.id()),
                Some(piece_type)
            );
        }
        for (id, expected) in [
            ("btp", Some(StructurePieceType::BuriedTreasure)),
            ("BTP", Some(StructurePieceType::BuriedTreasure)),
            ("minecraft:OMB", Some(StructurePieceType::OceanMonumentBase)),
            ("nvi", Some(StructurePieceType::Jigsaw)),
            ("minecraft:pcp", Some(StructurePieceType::Jigsaw)),
            ("BastionRemnant", Some(StructurePieceType::Jigsaw)),
            ("runtime", Some(StructurePieceType::Jigsaw)),
            ("othermod:btp", None),
            ("", None),
        ] {
            assert_eq!(StructurePieceType::from_id(id), expected, "{id}");
        }
    }
}
