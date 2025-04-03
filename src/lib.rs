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

// Find the in check threats for the king given, returning the nonces of
// the threats.
fn in_check_threats(board: &Board, row_size: u32, king_pos: u32) -> Vec<u32> {
    let (king_x, king_y) = pos_to_xy(row_size, king_pos);
    let mut threats = vec![];
    // The following code takes the position of the king, then searches for pieces in
    // positions that might threaten the king, then adding them as threats if they're
    // the kind of piece to be a threat.
    macro_rules! piece_add_threat_if_valid {
        ($piece:ident, $x:expr, $y:expr) => {
            if in_bounds(row_size, $x, $y) {
                if let Some((Piece::$piece, n)) = board.get(&xy_to_pos(row_size, $x, $y)) {
                    threats.push(*n);
                }
            }
        };
    }
    // Pawn
    for dx in [-1, 1] {
        let x = king_x.wrapping_add_signed(dx);
        let y = king_y.wrapping_sub(1);
        piece_add_threat_if_valid!(PAWN, x, y);
    }
    // Knight
    for (dx, dy) in [
        (-2, -1),
        (-2, 1),
        (-1, -2),
        (-1, 2),
        (1, -2),
        (1, 2),
        (2, -1),
        (2, 1),
    ] {
        piece_add_threat_if_valid!(
            KNIGHT,
            king_x.wrapping_add_signed(dx),
            king_y.wrapping_add_signed(dy)
        );
    }
    // Rook/Queen
    for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
        let mut x = king_x;
        let mut y = king_y;
        loop {
            x = x.wrapping_add_signed(dx);
            y = y.wrapping_add_signed(dy);
            // It's true that the macro does this check as well, but any compiler
            // would optimise this out, so we leave it for brevity reasons.
            if !in_bounds(row_size, x, y) {
                break;
            }
            piece_add_threat_if_valid!(CASTLE, x, y);
            piece_add_threat_if_valid!(QUEEN, x, y);
        }
    }
    // Bishop/Queen
    for (dx, dy) in [(-1, -1), (-1, 1), (1, -1), (1, 1)] {
        let mut x = king_x;
        let mut y = king_y;
        loop {
            x = x.wrapping_add_signed(dx);
            y = y.wrapping_add_signed(dy);
            if !in_bounds(row_size, x, y) {
                break;
            }
            piece_add_threat_if_valid!(BISHOP, x, y);
            piece_add_threat_if_valid!(QUEEN, x, y);
        }
    }
    // King
    for dx in [-1, 0, 1] {
        for dy in [-1, 0, 1] {
            // Make sure we're not hcecking the king against itself, and that we're
            // not in the corner.
            let x = king_x.wrapping_add_signed(dx);
            let y = king_y.wrapping_add_signed(dy);
            if dx == 0 && dy == 0 || xy_to_pos(row_size, x, y) == king_pos {
                continue;
            }
            piece_add_threat_if_valid!(KING, x, y);
        }
    }
    threats
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
        board.insert(pos, (p, i));
        if p == Piece::KING {
            last_king = Some((pos, i));
            threats = in_check_threats(&board, row_size, pos);
        } else if let Some((last_king_pos, last_king_nonce)) = last_king {
            {
                let (king_x, king_y) = pos_to_xy(row_size, last_king_pos);
                let (piece_x, piece_y) = pos_to_xy(row_size, pos);

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
                    continue;
                }

                let solved = match p {
                    Piece::PAWN => piece_y + 1 == king_y && dx == 1,
                    Piece::CASTLE => dx == 0 || dy == 0,
                    Piece::QUEEN => dx == 0 || dy == 0 || dx == dy,
                    Piece::BISHOP => dx == dy,
                    Piece::KNIGHT => dx * dy == 2,
                    Piece::KING => dx <= 1 && dy <= 1,
                };

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
