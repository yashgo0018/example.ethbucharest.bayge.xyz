// This contract is used as the reference via The Wizard on Stylus. It
// should be changed to compete in the hackathon!

extern crate alloc;

use stylus_sdk::{alloy_primitives::*, prelude::*};

use alloc::{collections::BTreeMap, vec, vec::Vec};

use libbucharesthashing::{immutables::*, prover, prover::Piece};

/* ~~~~~~~~~~~~~~ BOARD IMPLEMENTATION ~~~~~~~~~~~~~~ */

/// Board that this game is played on. Could be of any size. This could
/// be optimised for gas golfing.
pub type Board = BTreeMap<u32, (Piece, u32)>;

fn pos_to_xy(row_size: u32, p: u32) -> (u32, u32) {
    (p % row_size, p / row_size)
}

fn xy_to_pos(row_size: u32, x: u32, y: u32) -> u32 {
    y.wrapping_mul(row_size).wrapping_add(x)
}

fn in_bounds(row_size: u32, x: u32, y: u32) -> bool {
    x < row_size && y < row_size
}

fn in_check_threats(
    search_start: u32,
    search_end: u32,
    board: &Board,
    row_size: u32,
    king_pos: u32,
) -> Vec<u32> {
    let mut threats = vec![];
    for i in search_start..search_end {
        if let Some((piece, piece_pos)) = board.get(&i) {
            if is_solved(row_size, king_pos, *piece_pos, *piece) {
                threats.push(i);
            }
        }
    }
    threats
}

fn is_solved(row_size: u32, king_pos: u32, piece_pos: u32, piece: Piece) -> bool {
    let (king_x, king_y) = pos_to_xy(row_size, king_pos);
    let (piece_x, piece_y) = pos_to_xy(row_size, piece_pos);

    let dx = if king_x > piece_x {
        king_x - piece_x
    } else {
        piece_x - king_x
    };
    let dy = if king_y > piece_y {
        king_y - piece_y
    } else {
        piece_y - king_y
    };

    if dx + dy == 0 {
        return false;
    }

    match piece {
        Piece::PAWN => piece_y + 1 == king_y && dx == 1,
        Piece::CASTLE => dx == 0 || dy == 0,
        Piece::QUEEN => dx == 0 || dy == 0 || dx == dy,
        Piece::BISHOP => dx == dy,
        Piece::KNIGHT => dx * dy == 2,
        Piece::KING => dx <= 1 && dy <= 1,
    }
}

pub fn solve(starting_hash: &[u8], start: u32) -> Option<(u32, u32)> {
    let row_size = BOARD_SIZE.isqrt();
    let mut board = BTreeMap::new();
    let mut last_king = None;
    let mut threats = vec![];
    for i in start..MAX_TRIES {
        let e = prover::hash(starting_hash, i);
        let king_id: u8 = Piece::KING.into();
        let p_id: u8 = (e % (king_id as u64 + 1)).try_into().unwrap();
        let p = Piece::try_from(p_id).unwrap();
        let offset: u32 = (e >> 32).try_into().unwrap();
        let pos: u32 = offset % BOARD_SIZE;
        board.insert(i, (p, pos));
        if p == Piece::KING {
            last_king = Some((pos, i));
            threats = in_check_threats(start, i, &board, row_size, pos);
        } else if let Some((last_king_pos, last_king_nonce)) = last_king {
            {
                let solved = is_solved(row_size, last_king_pos, pos, p);

                if solved {
                    threats.push(i);
                }
            }
        }

        if let Some((_last_king_pos, last_king_nonce)) = last_king {
            if threats.len() >= CHECKS_NEEDED as usize {
                threats.push(last_king_nonce);
                let first_threat = *threats.iter().min().unwrap();
                println!("first_threat: {:?}", threats);
                return Some((first_threat, i));
            }
        }
    }
    None
}

/* ~~~~~~~~~~~~~~ CONTRACT ENTRYPOINT ~~~~~~~~~~~~~~ */

#[storage]
#[entrypoint]
pub struct Storage {}

#[public]
impl Storage {
    // We need to provide this function for the prover contract to check this
    // contract's performance with this function.
    pub fn prove(&self, hash: FixedBytes<32>, from: u32) -> Result<(u32, u32), Vec<u8>> {
        Ok(solve(hash.as_slice(), from).unwrap())
    }
}

/* ~~~~~~~~~~~~~~ ALGORITHM TESTING ~~~~~~~~~~~~~~ */

// This test code will randomly slam the function to test if it behaves
// consistently. It will create hashes for the test function.

#[cfg(all(test, not(target_arch = "wasm32")))]
mod test {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_solve(starting_hash in any::<[u8; 32]>()) {
            let y: u32 = 10;
            let a: i32 = -1;
            let x: u32 = y.wrapping_add_signed(a);
            assert_eq!(x, 9);
            println!("x: {}", CHECKS_NEEDED);
            for i in 0..10 {
                loop {
                    println!("i: {}", i);
                    break;
                }
            }
            // First, let's test if the user-defined algorithm is consistent.
            let (e_l, e_h) = solve(&starting_hash, 0).unwrap();
            // Let's run our function against the first invocation of the function!
            let (t_l, t_h) = solve(&starting_hash, e_l).unwrap();
            // Now let's check if it's consistent.
            assert_eq!((e_l, e_h), (t_l, t_h), "user contract not consistent. {e_l} != {t_l} or {e_h} != {t_h}");
            // Now, let's test if the remote contract's prove function is consistent with the
            // local function here.
            let (p_l, p_h) = prover::default_solve(&starting_hash, e_l).unwrap();
            assert_eq!(
                (e_l, e_h), (p_l, p_h),
                "user contract inconsistent with reference. {e_l} != {p_l} or {e_h} != {p_h}"
            );
        }
    }
}
