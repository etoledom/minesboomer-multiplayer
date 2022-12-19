use crate::SerializableCell;
use minesweeper_core::{Board, Cell, Point, Vec2};

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct SerializableBoard {
    pub data: Vec<Vec<SerializableCell>>,
}

impl SerializableBoard {
    pub fn new_from_json(str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(str)
    }

    pub fn to_json_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

impl From<Board> for SerializableBoard {
    fn from(board: Board) -> Self {
        let size = board.get_size();

        let rows: Vec<Vec<SerializableCell>> = populate_vec_with_closure(size.width, size.height, |x, y| {
            let cell = *board.cell_at(Point { x, y }).unwrap();
            cell.into()
        });

        SerializableBoard { data: rows }
    }
}

impl From<SerializableBoard> for Board {
    fn from(s_board: SerializableBoard) -> Self {
        let width = s_board.data.len();
        let height = s_board.data[0].len();

        let rows: Vec<Vec<Cell>> = populate_vec_with_closure(width, height, |x, y| s_board.data[x][y].clone().into());

        let cells = Vec2::new(rows);
        Board::new_with_cells(cells)
    }
}

fn populate_vec_with_closure<T, F>(width: usize, height: usize, closure: F) -> Vec<Vec<T>>
where
    F: Fn(usize, usize) -> T,
{
    let mut rows = Vec::<Vec<T>>::with_capacity(width);
    for x in 0..width {
        let mut columns = Vec::<T>::with_capacity(height);
        for y in 0..height {
            let cell = closure(x, y);

            columns.push(cell);
        }
        rows.push(columns);
    }
    rows
}

#[cfg(test)]
mod tests {
    use crate::SerializablePoint;
    use minesweeper_core::Size;

    use super::*;

    #[test]
    fn test_s_board_from_board() {
        let board = Board::new(2, Size { width: 3, height: 5 });
        let s_board: SerializableBoard = board.into();

        assert_eq!(s_board.data.len(), 3);
        assert_eq!(s_board.data[0].len(), 5);
        assert_eq!(s_board.data[1][3].coordinates.x, 1);
        assert_eq!(s_board.data[1][3].coordinates.y, 3);
    }

    #[test]
    fn test_board_from_s_board() {
        let cells = populate_vec_with_closure(3, 5, |x, y| SerializableCell {
            number: 0,
            cleared: false,
            flagged: false,
            coordinates: SerializablePoint { x, y },
        });

        let s_board = SerializableBoard { data: cells };
        let board: Board = s_board.into();

        assert_eq!(board.get_width(), 3);
        assert_eq!(board.get_height(), 5);
        assert_eq!(board.cell_at(Point { x: 1, y: 3 }).unwrap().coordinates.x, 1);
        assert_eq!(board.cell_at(Point { x: 1, y: 3 }).unwrap().coordinates.y, 3);
    }

    #[test]
    fn test_s_board_to_string() {
        let cells = populate_vec_with_closure(3, 2, |x, y| SerializableCell {
            number: x as i8 - 1,
            cleared: x.is_power_of_two(),
            flagged: x.is_power_of_two(),
            coordinates: SerializablePoint { x, y },
        });

        let s_board = SerializableBoard { data: cells };
        let board = s_board.to_json_string();

        assert_eq!(board, "{\"data\":[[{\"number\":-1,\"cleared\":false,\"flagged\":false,\"coordinates\":{\"x\":0,\"y\":0}},{\"number\":-1,\"cleared\":false,\"flagged\":false,\"coordinates\":{\"x\":0,\"y\":1}}],[{\"number\":0,\"cleared\":true,\"flagged\":true,\"coordinates\":{\"x\":1,\"y\":0}},{\"number\":0,\"cleared\":true,\"flagged\":true,\"coordinates\":{\"x\":1,\"y\":1}}],[{\"number\":1,\"cleared\":true,\"flagged\":true,\"coordinates\":{\"x\":2,\"y\":0}},{\"number\":1,\"cleared\":true,\"flagged\":true,\"coordinates\":{\"x\":2,\"y\":1}}]]}");
    }

    #[test]
    fn test_string_to_s_board() {
        let str = "{\"data\":[[{\"number\":-1,\"cleared\":false,\"flagged\":false,\"coordinates\":{\"x\":0,\"y\":0}},{\"number\":-1,\"cleared\":false,\"flagged\":false,\"coordinates\":{\"x\":0,\"y\":1}}],[{\"number\":0,\"cleared\":true,\"flagged\":true,\"coordinates\":{\"x\":1,\"y\":0}},{\"number\":0,\"cleared\":true,\"flagged\":true,\"coordinates\":{\"x\":1,\"y\":1}}],[{\"number\":1,\"cleared\":true,\"flagged\":true,\"coordinates\":{\"x\":2,\"y\":0}},{\"number\":1,\"cleared\":true,\"flagged\":true,\"coordinates\":{\"x\":2,\"y\":1}}]]}";
        let board = SerializableBoard::new_from_json(str).unwrap();

        assert_eq!(board.data.len(), 3);
        assert_eq!(board.data[0].len(), 2);
        assert!(board.data[1][1].flagged);
    }
}
