use std::cmp::Ordering;

pub enum InsertError {
    PacketExpired,
    Duplicate,
    WrappedAround,
}

#[derive(Debug)]
pub struct ReliabilityBuffer<T, const N: usize> {
    items: [Option<(u64, T)>; N],
    next_slot: usize,
    read_pos: usize,
    next_sequence_number: u64,
}

impl<T: std::fmt::Display, const N: usize> Default for ReliabilityBuffer<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: std::fmt::Display, const N: usize> ReliabilityBuffer<T, N> {
    pub fn new() -> Self {
        Self {
            items: std::array::from_fn(|_| None),
            next_slot: 0,
            read_pos: 0,
            next_sequence_number: 0,
        }
    }

    pub fn insert(&mut self, new_sequence_number: u64, item: T) {
        println!();
        println!("Insert {item} at {new_sequence_number}");
        let new_slot = new_sequence_number as usize % N;

        match &self.items[new_slot] {
            Some((existing_sequence_number, _item)) => {
                match new_sequence_number.cmp(existing_sequence_number) {
                    Ordering::Less => {
                        // return Err(InsertError::Expired)
                        println!("Expired!");
                    },
                    Ordering::Equal => {
                        // return Err(InsertError::Duplicate)
                        println!("Duplicate!");
                    },
                    Ordering::Greater => {
                        // return Err(InsertError::WrappedAround)
                        println!("WrappedAround!");
                    },
                }
            },
            None => {
                if new_sequence_number as usize >= self.next_sequence_number as usize + N {
                    println!("WrappedAround!");
                    return;
                }

                if new_sequence_number as usize + N <= self.next_sequence_number as usize {
                    println!("Expired!");
                    return;
                }

                self.items[new_slot] = Some((new_sequence_number, item));

                if new_slot == self.next_slot {
                    self.next_slot = (self.next_slot + 1) % N;
                }

                // iterate from read_pos until we hit None
                while let Some((sequence_number, msg)) = self.items[self.read_pos].take() {
                    println!("\tReturn {msg}");
                    self.read_pos = (self.read_pos + 1) % N;
                    self.next_sequence_number = sequence_number + 1;
                }
            },
        }
    }

    pub fn reset(&mut self) {
        self.items = std::array::from_fn(|_| None);
        self.next_slot = 0;
        self.read_pos = 0;
        self.next_sequence_number = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let mut buffer: ReliabilityBuffer<&str, 5> = ReliabilityBuffer::new();
        println!("{:?}", buffer);

        buffer.insert(0, "0"); // We get ["hello"]
        println!("{:?}", buffer);

        buffer.insert(1, "1"); // We get ["world"]
        println!("{:?}", buffer);

        buffer.insert(3, "3"); // We get nothing
        println!("{:?}", buffer);

        buffer.insert(2, "2"); // We get ["good", "day"]
        println!("{:?}", buffer);

        buffer.insert(6, "6"); // We get nothing
        println!("{:?}", buffer);

        buffer.insert(5, "5"); // We get nothing
        println!("{:?}", buffer);

        buffer.insert(4, "4"); // We get ["hey", "cool", "hat"]
        println!("{:?}", buffer);

        buffer.insert(11, "11");
        println!("{:?}", buffer);

        buffer.insert(10, "10");
        println!("{:?}", buffer);

        buffer.insert(9, "9");
        println!("{:?}", buffer);

        buffer.insert(8, "8");
        println!("{:?}", buffer);

        buffer.insert(7, "7");
        println!("{:?}", buffer);

        buffer.insert(16, "16");
        println!("{:?}", buffer);

        buffer.insert(12, "12");
        println!("{:?}", buffer);

        buffer.insert(15, "15");
        println!("{:?}", buffer);

        buffer.insert(15, "15");
        println!("{:?}", buffer);

        buffer.insert(15, "15");
        println!("{:?}", buffer);

        buffer.insert(14, "14");
        println!("{:?}", buffer);

        buffer.insert(13, "13");
        println!("{:?}", buffer);

        buffer.insert(2, "2");
        println!("{:?}", buffer);
    }
}
