use std::cmp::Ordering;

#[derive(Debug, PartialEq)]
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

    pub fn insert(
        &mut self,
        new_sequence_number: u64,
        item: T,
    ) -> Result<ReliabilityBufferIterator<T, N>, InsertError> {
        println!();
        println!("Insert {item} at {new_sequence_number}");
        let new_slot = new_sequence_number as usize % N;

        match &self.items[new_slot] {
            Some((existing_sequence_number, _item)) => {
                match new_sequence_number.cmp(existing_sequence_number) {
                    Ordering::Less => {
                        Err(InsertError::PacketExpired)
                        // println!("Expired!");
                    },
                    Ordering::Equal => {
                        Err(InsertError::Duplicate)
                        // println!("Duplicate!");
                    },
                    Ordering::Greater => {
                        Err(InsertError::WrappedAround)
                        // println!("WrappedAround!");
                    },
                }
            },
            None => {
                if new_sequence_number as usize >= self.next_sequence_number as usize + N {
                    // println!("WrappedAround!");
                    return Err(InsertError::WrappedAround);
                }

                if new_sequence_number as usize + N <= self.next_sequence_number as usize {
                    // println!("Expired!");
                    return Err(InsertError::PacketExpired);
                }

                self.items[new_slot] = Some((new_sequence_number, item));

                if new_slot == self.next_slot {
                    self.next_slot = (self.next_slot + 1) % N;
                }

                Ok(ReliabilityBufferIterator { buffer: self })

                // // iterate from read_pos until we hit None
                // while let Some((sequence_number, msg)) = self.items[self.read_pos].take() {
                //     println!("\tReturn {msg}");
                //     self.read_pos = (self.read_pos + 1) % N;
                //     self.next_sequence_number = sequence_number + 1;
                // }
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

#[derive(Debug)]
pub struct ReliabilityBufferIterator<'a, T, const N: usize> {
    buffer: &'a mut ReliabilityBuffer<T, N>,
}

impl<T, const N: usize> Iterator for ReliabilityBufferIterator<'_, T, N> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        self.buffer.items[self.buffer.read_pos].take().map(|(sequence_number, msg)| {
            self.buffer.read_pos = (self.buffer.read_pos + 1) % N;
            self.buffer.next_sequence_number = sequence_number + 1;
            msg
        })
        // // iterate from read_pos until we hit None
        // while let Some((sequence_number, msg)) = self.items[self.read_pos].take() {
        //     println!("\tReturn {msg}");
        //     self.read_pos = (self.read_pos + 1) % N;
        //     self.next_sequence_number = sequence_number + 1;
        // }
        // None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let mut buffer: ReliabilityBuffer<&str, 5> = ReliabilityBuffer::new();
        println!("{:?}", buffer);

        for msg in buffer.insert(0, "0").unwrap() {
            println!("\t{msg}");
        }

        for msg in buffer.insert(1, "1").unwrap() {
            println!("\t{msg}");
        }

        for msg in buffer.insert(3, "3").unwrap() {
            println!("\t{msg}");
        }

        for msg in buffer.insert(2, "2").unwrap() {
            println!("\t{msg}");
        }

        for msg in buffer.insert(6, "6").unwrap() {
            println!("\t{msg}");
        }

        for msg in buffer.insert(5, "5").unwrap() {
            println!("\t{msg}");
        }

        for msg in buffer.insert(4, "4").unwrap() {
            println!("\t{msg}");
        }

        for msg in buffer.insert(11, "11").unwrap() {
            println!("\t{msg}");
        }

        for msg in buffer.insert(10, "10").unwrap() {
            println!("\t{msg}");
        }

        for msg in buffer.insert(9, "9").unwrap() {
            println!("\t{msg}");
        }

        for msg in buffer.insert(8, "8").unwrap() {
            println!("\t{msg}");
        }

        for msg in buffer.insert(7, "7").unwrap() {
            println!("\t{msg}");
        }

        for msg in buffer.insert(16, "16").unwrap() {
            println!("\t{msg}");
        }

        for msg in buffer.insert(12, "12").unwrap() {
            println!("\t{msg}");
        }

        for msg in buffer.insert(15, "15").unwrap() {
            println!("\t{msg}");
        }

        assert_eq!(buffer.insert(15, "15").unwrap_err(), InsertError::Duplicate);
        assert_eq!(buffer.insert(15, "15").unwrap_err(), InsertError::Duplicate);

        for msg in buffer.insert(14, "14").unwrap() {
            println!("\t{msg}");
        }

        for msg in buffer.insert(13, "13").unwrap() {
            println!("\t{msg}");
        }

        assert_eq!(buffer.insert(2, "2").unwrap_err(), InsertError::PacketExpired);
    }
}
