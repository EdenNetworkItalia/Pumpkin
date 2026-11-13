use pumpkin_data::Block;
use pumpkin_util::{
    BlockDirection,
    math::block_box::BlockBox,
    random::{RandomGenerator, RandomImpl, legacy_rand::LegacyRand},
};

use crate::{
    ProtoChunk,
    generation::structure::{
        piece::StructurePieceType,
        structures::{
            StructurePiece, StructurePieceBase, StructurePiecesCollector,
            nether_fortress::NetherFortressPiece,
        },
    },
    world::WorldPortalExt,
};

/// Dead-end / terminator piece for the exterior bridge (5 × 10 × 8).
/// Generated randomly on each side and used as a fallback when no valid piece
/// can be placed.
pub struct BridgeEndPiece {
    pub piece: NetherFortressPiece,
    /// Seed used so the crumbling geometry is deterministic per world seed.
    pub seed: u32,
}

impl BridgeEndPiece {
    pub fn create(
        collector: &mut StructurePiecesCollector,
        random: &mut impl RandomImpl,
        x: i32,
        y: i32,
        z: i32,
        orientation: BlockDirection,
        chain_length: u32,
    ) -> Option<Box<dyn StructurePieceBase>> {
        let bbox = BlockBox::rotated(x, y, z, -1, -3, 0, 5, 10, 8, &orientation);
        if !NetherFortressPiece::is_in_bounds(&bbox) || collector.get_intersecting(&bbox).is_some()
        {
            return None;
        }

        let seed = random.next_i32() as u32;
        let mut piece = NetherFortressPiece::new(
            StructurePieceType::NetherFortressBridgeEnd,
            chain_length,
            bbox,
        );
        piece.piece.set_facing(Some(orientation));
        Some(Box::new(Self { piece, seed }))
    }

    /// The random that crumbles the bridge end. Vanilla
    /// `RandomSource.createThreadLocalInstance(selfSeed)` widens the int seed with its sign.
    const fn crumble_random(&self) -> LegacyRand {
        LegacyRand::from_seed(self.seed as i32 as i64 as u64)
    }
}

impl StructurePieceBase for BridgeEndPiece {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn get_structure_piece(&self) -> &StructurePiece {
        &self.piece.piece
    }

    fn get_structure_piece_mut(&mut self) -> &mut StructurePiece {
        &mut self.piece.piece
    }

    fn place(
        &mut self,
        chunk: &mut ProtoChunk,
        _block_registry: &dyn WorldPortalExt,
        _random: &mut RandomGenerator,
        _seed: i64,
        chunk_box: &BlockBox,
    ) {
        let bb = *chunk_box;
        let p = &self.piece.piece;
        let nb = Block::NETHER_BRICKS.default_state;

        // Use a seeded local random so geometry is deterministic.
        let mut rng = self.crumble_random();

        for i in 0..=4i32 {
            for j in 3..=4i32 {
                let k = rng.next_bounded_i32(8);
                p.fill_with_outline(chunk, &bb, false, i, j, 0, i, j, k, nb, nb);
            }
        }

        let k0 = rng.next_bounded_i32(8);
        p.fill_with_outline(chunk, &bb, false, 0, 5, 0, 0, 5, k0, nb, nb);

        let k1 = rng.next_bounded_i32(8);
        p.fill_with_outline(chunk, &bb, false, 4, 5, 0, 4, 5, k1, nb, nb);

        for i in 0..=4i32 {
            let j = rng.next_bounded_i32(5);
            p.fill_with_outline(chunk, &bb, false, i, 2, 0, i, 2, j, nb, nb);
        }

        for i in 0..=4i32 {
            for j in 0..=1i32 {
                let k = rng.next_bounded_i32(3);
                p.fill_with_outline(chunk, &bb, false, i, j, 0, i, j, k, nb, nb);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The first values of `new SingleThreadedRandomSource(-123456789).nextInt(8)` in vanilla.
    #[test]
    fn bridge_end_random_matches_vanilla() {
        let piece = BridgeEndPiece {
            piece: NetherFortressPiece::new(
                StructurePieceType::NetherFortressBridgeEnd,
                0,
                BlockBox::new(0, 0, 0, 4, 9, 7),
            ),
            seed: (-123_456_789i32) as u32,
        };
        let mut random = piece.crumble_random();
        let values: Vec<i32> = (0..10).map(|_| random.next_bounded_i32(8)).collect();
        assert_eq!(values, [2, 6, 7, 5, 3, 3, 6, 5, 0, 2]);
    }
}
