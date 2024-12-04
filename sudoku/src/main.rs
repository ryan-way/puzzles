#![feature(iter_array_chunks)]
#![feature(array_chunks)]

use std::{collections::HashSet, fmt::Display, str::FromStr};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum CellValue {
    EMPTY,
    ONE,
    TWO,
    THREE,
    FOUR,
    FIVE,
    SIX,
    SEVEN,
    EIGHT,
    NINE,
}

impl Display for CellValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            CellValue::EMPTY => "-",
            CellValue::ONE => "1",
            CellValue::TWO => "2",
            CellValue::THREE => "3",
            CellValue::FOUR => "4",
            CellValue::FIVE => "5",
            CellValue::SIX => "6",
            CellValue::SEVEN => "7",
            CellValue::EIGHT => "8",
            CellValue::NINE => "9",
        };

        f.write_str(s)
    }
}

#[derive(Clone, Debug)]
struct CellIndex {
    x: usize,
    y: usize,
}

impl CellIndex {
    fn new(x: usize, y: usize) -> Self {
        CellIndex { x, y }
    }
}

#[derive(Clone, Debug)]
struct CellFlatIndex {
    idx: usize,
}

impl CellFlatIndex {
    fn new(idx: usize) -> Self {
        CellFlatIndex { idx }
    }
}

#[derive(Clone, Debug)]
struct RowIndex {
    idx: usize,
}

impl RowIndex {
    fn new(idx: usize) -> Self {
        RowIndex { idx }
    }
}

#[derive(Clone, Debug)]
struct ColumnIndex {
    idx: usize,
}

impl ColumnIndex {
    fn new(idx: usize) -> Self {
        ColumnIndex { idx }
    }
}

#[derive(Clone, Debug)]
struct SubgridIndex {
    idx: usize,
}

impl SubgridIndex {
    fn new(idx: usize) -> Self {
        SubgridIndex { idx }
    }
}

impl Into<CellFlatIndex> for CellIndex {
    fn into(self) -> CellFlatIndex {
        CellFlatIndex {
            idx: self.x + self.y * 9,
        }
    }
}

impl Into<RowIndex> for CellIndex {
    fn into(self) -> RowIndex {
        RowIndex { idx: self.y }
    }
}

impl Into<ColumnIndex> for CellIndex {
    fn into(self) -> ColumnIndex {
        ColumnIndex { idx: self.x }
    }
}

impl Into<SubgridIndex> for CellIndex {
    fn into(self) -> SubgridIndex {
        SubgridIndex {
            idx: (self.y / 3) * 3 + self.x / 3,
        }
    }
}

trait Index {
    fn cells(&self) -> Vec<CellIndex>;
}

impl Index for RowIndex {
    fn cells(&self) -> Vec<CellIndex> {
        (0..9).map(|idx| CellIndex::new(idx, self.idx)).collect()
    }
}

impl Index for ColumnIndex {
    fn cells(&self) -> Vec<CellIndex> {
        (0..9).map(|idx| CellIndex::new(self.idx, idx)).collect()
    }
}

impl Index for SubgridIndex {
    fn cells(&self) -> Vec<CellIndex> {
        let root_x = (self.idx % 3) * 3;
        let root_y = (self.idx / 3) * 3;
        (0..9)
            .map(|idx| CellIndex::new(root_x + (idx % 3), root_y + (idx / 3)))
            .collect()
    }
}

struct Cell<'a> {
    puzzle: &'a Puzzle,
    idx: CellIndex,
}

impl<'a> Cell<'a> {
    fn new(puzzle: &'a Puzzle, idx: CellIndex) -> Self {
        Cell { puzzle, idx }
    }

    fn value(&self) -> CellValue {
        self.puzzle.0[self.idx.y][self.idx.x]
    }

    fn row(&self) -> Section<'_, RowIndex> {
        self.puzzle.get_row(self.idx.clone().into())
    }

    fn col(&self) -> Section<'_, ColumnIndex> {
        self.puzzle.get_col(self.idx.clone().into())
    }

    fn subgrid(&self) -> Section<'_, SubgridIndex> {
        self.puzzle.get_subgrid(self.idx.clone().into())
    }

    fn get_eliminated_values(&self) -> Vec<CellValue> {
        vec![
            self.row().nonempty_cells(),
            self.col().nonempty_cells(),
            self.subgrid().nonempty_cells(),
        ]
        .into_iter()
        .flatten()
        .map(|cell| cell.value())
        .collect::<HashSet<CellValue>>()
        .into_iter()
        .collect()
    }

    fn get_possible_values(&self) -> Vec<CellValue> {
        let complete: HashSet<CellValue> = COMPLETE.iter().skip(1).copied().collect();
        let eliminated: HashSet<CellValue> = self.get_eliminated_values().into_iter().collect();
        complete.difference(&eliminated).copied().collect()
    }
}

#[derive(Debug)]
struct Section<'a, T>
where
    T: Index,
{
    puzzle: &'a Puzzle,
    idx: T,
}

impl<'a, T> Section<'a, T>
where
    T: Index + std::fmt::Debug,
{
    fn new(puzzle: &'a Puzzle, idx: T) -> Self {
        Section { puzzle, idx }
    }

    fn cells(&self) -> Vec<Cell> {
        self.idx
            .cells()
            .into_iter()
            .map(|idx| self.puzzle.get_cell(idx))
            .collect()
    }

    fn nonempty_cells(&self) -> Vec<Cell> {
        self.cells()
            .into_iter()
            .filter(|cell| cell.value() != CellValue::EMPTY)
            .collect()
    }

    fn empty_cells(&self) -> Vec<Cell> {
        self.cells()
            .into_iter()
            .filter(|cell| cell.value() == CellValue::EMPTY)
            .collect()
    }

    fn is_valid(&self) -> bool {
        let set: HashSet<CellValue> = self
            .cells()
            .into_iter()
            .map(|cell| cell.value())
            .filter(|value| value != &CellValue::EMPTY)
            .collect();

        let values: Vec<CellValue> = self
            .cells()
            .into_iter()
            .map(|cell| cell.value())
            .filter(|value| value != &CellValue::EMPTY)
            .collect();

        set.len() == values.len()
    }

    fn is_complete(&self) -> bool {
        let set: HashSet<CellValue> = self.cells().into_iter().map(|cell| cell.value()).collect();

        set.get(&CellValue::EMPTY).is_none() && set.len() == 9
    }
}

#[derive(Debug)]
struct Puzzle([[CellValue; 9]; 9]);

impl Puzzle {
    fn get_cell(&self, idx: CellIndex) -> Cell {
        Cell::new(self, idx)
    }

    fn get_cells(&self) -> Vec<Cell> {
        (0..9)
            .flat_map(|y| (0..9).map(move |x| CellIndex::new(x, y)))
            .map(|idx| Cell::new(self, idx))
            .collect()
    }

    fn get_nonempty_cells(&self) -> Vec<Cell> {
        self.get_cells()
            .into_iter()
            .filter(|cell| cell.value() != CellValue::EMPTY)
            .collect()
    }

    fn get_empty_cells(&self) -> Vec<Cell> {
        self.get_cells()
            .into_iter()
            .filter(|cell| cell.value() == CellValue::EMPTY)
            .collect()
    }

    fn set_cell(&mut self, idx: CellIndex, value: CellValue) {
        self.0[idx.y][idx.x] = value;
    }

    fn get_row(&self, idx: RowIndex) -> Section<'_, RowIndex> {
        Section::new(self, idx)
    }

    fn get_rows(&self) -> Vec<Section<'_, RowIndex>> {
        (0..9)
            .map(RowIndex::new)
            .map(|idx| self.get_row(idx))
            .collect()
    }

    fn get_col(&self, idx: ColumnIndex) -> Section<'_, ColumnIndex> {
        Section::new(self, idx)
    }

    fn get_cols(&self) -> Vec<Section<'_, ColumnIndex>> {
        (0..9)
            .map(ColumnIndex::new)
            .map(|idx| self.get_col(idx))
            .collect()
    }

    fn get_subgrid(&self, idx: SubgridIndex) -> Section<'_, SubgridIndex> {
        Section::new(self, idx)
    }

    fn get_subgrids(&self) -> Vec<Section<'_, SubgridIndex>> {
        (0..9)
            .map(SubgridIndex::new)
            .map(|idx| self.get_subgrid(idx))
            .collect()
    }

    fn is_valid(&self) -> bool {
        self.get_rows().into_iter().all(|row| row.is_valid())
            && self.get_cols().into_iter().all(|col| col.is_valid())
            && self
                .get_subgrids()
                .into_iter()
                .all(|subgrid| subgrid.is_valid())
    }

    fn is_complete(&self) -> bool {
        self.get_rows().into_iter().all(|row| row.is_complete())
            && self.get_cols().into_iter().all(|col| col.is_complete())
            && self
                .get_subgrids()
                .into_iter()
                .all(|subgrid| subgrid.is_complete())
    }
}

impl FromStr for Puzzle {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let char_grid = s
            .lines()
            .flat_map(|row| {
                row.chars()
                    .array_chunks::<9>()
                    .next()
                    .ok_or("Wrong number of cols".to_owned())
            })
            .array_chunks::<9>()
            .next()
            .ok_or("Wrong number of rows".to_owned())?;

        Ok(Puzzle(char_grid.map(|row| {
            row.map(|c| match c {
                '9' => CellValue::NINE,
                '8' => CellValue::EIGHT,
                '7' => CellValue::SEVEN,
                '6' => CellValue::SIX,
                '5' => CellValue::FIVE,
                '4' => CellValue::FOUR,
                '3' => CellValue::THREE,
                '2' => CellValue::TWO,
                '1' => CellValue::ONE,
                _ => CellValue::EMPTY,
            })
        })))
    }
}

#[derive(Debug)]
struct Assignment {
    idx: CellIndex,
    value: CellValue,
}

fn last_possible(puzzle: &Puzzle) -> Vec<Assignment> {
    puzzle
        .get_empty_cells()
        .into_iter()
        .flat_map(|cell| {
            let possible = cell.get_possible_values();

            if possible.len() != 1 {
                None
            } else {
                let assignment = Assignment {
                    idx: cell.idx.clone(),
                    value: *possible.iter().next().unwrap(),
                };
                println!("Assignment from last possible: {:?}", assignment);
                Some(assignment)
            }
        })
        .collect()
}

fn last_remaining(puzzle: &Puzzle) -> Vec<Assignment> {
    let mut assignments = vec![];
    for subgrid in puzzle.get_subgrids() {
        for value in COMPLETE.iter().skip(1) {
            let possible_cells: Vec<Cell> = subgrid
                .empty_cells()
                .into_iter()
                .filter(|cell| cell.get_possible_values().contains(value))
                .collect();

            if possible_cells.len() != 1 {
                continue;
            }

            if let Some(cell) = possible_cells.first() {
                let assignment = Assignment {
                    idx: cell.idx.clone(),
                    value: value.clone(),
                };
                println!("Assignment from last remaining: {:?}", assignment);
                assignments.push(assignment);
            }
        }
    }
    assignments
}

struct Solver {
    puzzle: Puzzle,
}

static COMPLETE: [CellValue; 10] = [
    CellValue::EMPTY,
    CellValue::ONE,
    CellValue::TWO,
    CellValue::THREE,
    CellValue::FOUR,
    CellValue::FIVE,
    CellValue::SIX,
    CellValue::SEVEN,
    CellValue::EIGHT,
    CellValue::NINE,
];

impl Solver {
    pub fn new() -> Self {
        Solver {
            puzzle: Puzzle([[CellValue::EMPTY; 9]; 9]),
        }
    }

    pub fn from(puzzle: Puzzle) -> Self {
        Solver { puzzle }
    }

    pub fn solve(&mut self) {
        let mut change = true;
        while change {
            change = false;
            let assignments: Vec<Assignment> =
                vec![last_possible(&self.puzzle), last_remaining(&self.puzzle)]
                    .into_iter()
                    .flatten()
                    .collect();

            println!("Number of Assignments: {}", assignments.len());

            change |= assignments.len() != 0;

            for assignment in assignments {
                self.puzzle.set_cell(assignment.idx, assignment.value)
            }
        }
    }
}

fn main() {
    let puzzle: Puzzle = include_str!("puzzles/medium/1/input.txt").parse().unwrap();
    let mut solver: Solver = Solver::from(puzzle);

    solver.solve();

    for (idx, row) in solver.puzzle.0.iter().enumerate() {
        if idx % 3 == 0 && idx != 0 {
            println!();
        }
        let format = row
            .iter()
            .map(|value| format!("{}", value))
            .collect::<Vec<String>>()
            .array_chunks::<3>()
            .map(|chunk| chunk.join(""))
            .collect::<Vec<String>>()
            .join(" ");
        println!("{}", format);
    }

    println!("Valid: {}", solver.puzzle.is_valid());
    println!("Complete: {}", solver.puzzle.is_complete());
}

#[cfg(test)]
mod tests {
    use super::*;

    mod grid {
        use super::*;

        #[test]
        fn test_rows() {
            let puzzle: Puzzle = include_str!("puzzles/easy/1/input.txt").parse().unwrap();

            assert_eq!(
                puzzle
                    .get_rows()
                    .into_iter()
                    .map(|row| row.cells().into_iter().map(|cell| cell.value()).collect())
                    .collect::<Vec<Vec<CellValue>>>(),
                vec![
                    vec![
                        CellValue::EIGHT,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::FIVE,
                        CellValue::EMPTY,
                        CellValue::FOUR,
                        CellValue::SEVEN,
                        CellValue::EMPTY,
                        CellValue::TWO
                    ],
                    vec![
                        CellValue::NINE,
                        CellValue::THREE,
                        CellValue::TWO,
                        CellValue::SEVEN,
                        CellValue::EMPTY,
                        CellValue::EIGHT,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::EMPTY
                    ],
                    vec![
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::ONE,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::NINE,
                        CellValue::EIGHT,
                        CellValue::EMPTY
                    ],
                    vec![
                        CellValue::EMPTY,
                        CellValue::FIVE,
                        CellValue::FOUR,
                        CellValue::THREE,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::EMPTY
                    ],
                    vec![
                        CellValue::EMPTY,
                        CellValue::TWO,
                        CellValue::EMPTY,
                        CellValue::SIX,
                        CellValue::EMPTY,
                        CellValue::NINE,
                        CellValue::EMPTY,
                        CellValue::FIVE,
                        CellValue::EMPTY
                    ],
                    vec![
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::FIVE,
                        CellValue::EIGHT,
                        CellValue::FOUR,
                        CellValue::EMPTY
                    ],
                    vec![
                        CellValue::EMPTY,
                        CellValue::ONE,
                        CellValue::THREE,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::SIX,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::EMPTY
                    ],
                    vec![
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::FOUR,
                        CellValue::EMPTY,
                        CellValue::TWO,
                        CellValue::SIX,
                        CellValue::NINE,
                        CellValue::THREE
                    ],
                    vec![
                        CellValue::SIX,
                        CellValue::EMPTY,
                        CellValue::NINE,
                        CellValue::EIGHT,
                        CellValue::EMPTY,
                        CellValue::SEVEN,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::FIVE
                    ],
                ]
            )
        }

        #[test]
        fn test_cols() {
            let puzzle: Puzzle = include_str!("puzzles/easy/1/input.txt").parse().unwrap();

            assert_eq!(
                puzzle
                    .get_cols()
                    .into_iter()
                    .map(|row| row.cells().into_iter().map(|cell| cell.value()).collect())
                    .collect::<Vec<Vec<CellValue>>>(),
                vec![
                    vec![
                        CellValue::EIGHT,
                        CellValue::NINE,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::SIX
                    ],
                    vec![
                        CellValue::EMPTY,
                        CellValue::THREE,
                        CellValue::EMPTY,
                        CellValue::FIVE,
                        CellValue::TWO,
                        CellValue::EMPTY,
                        CellValue::ONE,
                        CellValue::EMPTY,
                        CellValue::EMPTY
                    ],
                    vec![
                        CellValue::EMPTY,
                        CellValue::TWO,
                        CellValue::EMPTY,
                        CellValue::FOUR,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::THREE,
                        CellValue::EMPTY,
                        CellValue::NINE
                    ],
                    vec![
                        CellValue::FIVE,
                        CellValue::SEVEN,
                        CellValue::ONE,
                        CellValue::THREE,
                        CellValue::SIX,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::FOUR,
                        CellValue::EIGHT
                    ],
                    vec![
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::EMPTY
                    ],
                    vec![
                        CellValue::FOUR,
                        CellValue::EIGHT,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::NINE,
                        CellValue::FIVE,
                        CellValue::SIX,
                        CellValue::TWO,
                        CellValue::SEVEN
                    ],
                    vec![
                        CellValue::SEVEN,
                        CellValue::EMPTY,
                        CellValue::NINE,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::EIGHT,
                        CellValue::EMPTY,
                        CellValue::SIX,
                        CellValue::EMPTY
                    ],
                    vec![
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::EIGHT,
                        CellValue::EMPTY,
                        CellValue::FIVE,
                        CellValue::FOUR,
                        CellValue::EMPTY,
                        CellValue::NINE,
                        CellValue::EMPTY
                    ],
                    vec![
                        CellValue::TWO,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::THREE,
                        CellValue::FIVE
                    ],
                ]
            )
        }

        #[test]
        fn test_sub_grids() {
            let puzzle: Puzzle = include_str!("puzzles/easy/1/input.txt").parse().unwrap();

            assert_eq!(
                puzzle
                    .get_subgrids()
                    .into_iter()
                    .map(|row| row.cells().into_iter().map(|cell| cell.value()).collect())
                    .collect::<Vec<Vec<CellValue>>>(),
                vec![
                    vec![
                        CellValue::EIGHT,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::NINE,
                        CellValue::THREE,
                        CellValue::TWO,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::EMPTY
                    ],
                    vec![
                        CellValue::FIVE,
                        CellValue::EMPTY,
                        CellValue::FOUR,
                        CellValue::SEVEN,
                        CellValue::EMPTY,
                        CellValue::EIGHT,
                        CellValue::ONE,
                        CellValue::EMPTY,
                        CellValue::EMPTY
                    ],
                    vec![
                        CellValue::SEVEN,
                        CellValue::EMPTY,
                        CellValue::TWO,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::NINE,
                        CellValue::EIGHT,
                        CellValue::EMPTY
                    ],
                    vec![
                        CellValue::EMPTY,
                        CellValue::FIVE,
                        CellValue::FOUR,
                        CellValue::EMPTY,
                        CellValue::TWO,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::EMPTY
                    ],
                    vec![
                        CellValue::THREE,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::SIX,
                        CellValue::EMPTY,
                        CellValue::NINE,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::FIVE
                    ],
                    vec![
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::FIVE,
                        CellValue::EMPTY,
                        CellValue::EIGHT,
                        CellValue::FOUR,
                        CellValue::EMPTY
                    ],
                    vec![
                        CellValue::EMPTY,
                        CellValue::ONE,
                        CellValue::THREE,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::SIX,
                        CellValue::EMPTY,
                        CellValue::NINE
                    ],
                    vec![
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::SIX,
                        CellValue::FOUR,
                        CellValue::EMPTY,
                        CellValue::TWO,
                        CellValue::EIGHT,
                        CellValue::EMPTY,
                        CellValue::SEVEN
                    ],
                    vec![
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::SIX,
                        CellValue::NINE,
                        CellValue::THREE,
                        CellValue::EMPTY,
                        CellValue::EMPTY,
                        CellValue::FIVE
                    ],
                ]
            )
        }
    }
}
