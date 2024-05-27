use std::{cmp::Ordering, iter::FusedIterator};

#[must_use]
#[derive(Debug, PartialEq)]
pub enum InsertResult {
    /// The message was successfully inserted.
    Inserted,

    /// The message has expired and cannot be buffered.
    Expired,

    /// The message has already been received.
    Duplicate,

    /// There is a packet missing but so many more recent
    /// messages have arrived that we can no longer buffer them.
    FullBuffer,
}

/// A buffer which can have messages inserted in any order, and
/// have them delivered in order as soon as possible, with no
/// duplicates and a configurable maximum number of messages to
/// buffer.
/// Mainly intended for receiving out-of-order and duplicate
/// network packets.
#[derive(Debug, PartialEq)]
pub struct OrderedBuffer<T, const N: usize> {
    items: [Option<(u64, T)>; N],
    read_pos: usize,
    next_sequence_number: u64,
}

impl<T, const N: usize> Default for OrderedBuffer<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

const fn assert_buffer_size(n: usize) {
    assert!(n > 0);
}

impl<T, const N: usize> OrderedBuffer<T, N> {
    const BUFFER_SIZE_CHECK: () = assert_buffer_size(N);

    pub fn new() -> Self {
        let () = Self::BUFFER_SIZE_CHECK;
        Self { items: std::array::from_fn(|_| None), read_pos: 0, next_sequence_number: 0 }
    }

    /// Inserts an item with a given sequence number.
    pub fn insert(&mut self, new_sequence_number: u64, item: T) -> InsertResult {
        let new_slot = new_sequence_number as usize % N;

        match &self.items[new_slot] {
            Some((existing_sequence_number, _item)) => {
                match new_sequence_number.cmp(existing_sequence_number) {
                    // TODO(bschwind) - I don't think we'll actually ever hit this case.
                    Ordering::Less => InsertResult::Expired,
                    // There is already a message here with the same sequence number.
                    Ordering::Equal => InsertResult::Duplicate,
                    // There's already a message here with a lower sequence number, this new
                    // one is so far ahead it wrapped around our items buffer.
                    Ordering::Greater => InsertResult::FullBuffer,
                }
            },
            None => {
                if new_sequence_number as usize >= self.next_sequence_number as usize + N {
                    // There is a free slot, but this sequence number is too far beyond the number
                    // of messages we can buffer.
                    return InsertResult::FullBuffer;
                }

                if new_sequence_number < self.next_sequence_number {
                    // `self.next_sequence_number` only advances when ordered messages are delivered
                    // to the caller, anything less has already been delivered.
                    return InsertResult::Duplicate;
                }

                self.items[new_slot] = Some((new_sequence_number, item));

                InsertResult::Inserted
            },
        }
    }

    /// Clears the buffer and resets all counters.
    pub fn reset(&mut self) {
        self.items = std::array::from_fn(|_| None);
        self.read_pos = 0;
        self.next_sequence_number = 0;
    }
}

impl<T, const N: usize> FusedIterator for &mut OrderedBuffer<T, N> {}

impl<T, const N: usize> Iterator for &mut OrderedBuffer<T, N> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        // Keep returning items while the item at our read_pos is Some(_).
        self.items[self.read_pos].take().map(|(sequence_number, msg)| {
            self.read_pos = (self.read_pos + 1) % N;
            self.next_sequence_number = sequence_number + 1;
            msg
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    trait Consume<T, const N: usize> {
        fn consume(&mut self) -> Vec<T>;
    }

    impl<T: Clone + std::fmt::Debug, const N: usize> Consume<T, N> for OrderedBuffer<T, N> {
        fn consume(&mut self) -> std::vec::Vec<T> {
            self.collect()
        }
    }

    #[test]
    fn it_works() {
        let mut buffer: OrderedBuffer<_, 5> = OrderedBuffer::new();

        // [_, _, _, _, _]
        let _ = buffer.insert(0, "0");
        // [0, _, _, _, _]
        assert_eq!(buffer.consume(), vec!["0"]);
        // [_, _, _, _, _]
        let _ = buffer.insert(1, "1");
        // [_, 1, _, _, _]
        assert_eq!(buffer.consume(), vec!["1"]);
        // [_, _, _, _, _]
        let _ = buffer.insert(3, "3");
        // [_, _, _, 3, _]
        assert!(buffer.consume().is_empty());
        // [_, _, _, 3, _]
        let _ = buffer.insert(2, "2");
        // [_, _, 2, 3, _]
        assert_eq!(buffer.consume(), vec!["2", "3"]);
        // [_, _, _, _, _]
        let _ = buffer.insert(6, "6");
        // [_, 6, _, _, _]
        assert!(buffer.consume().is_empty());
        // [_, 6, _, _, _]
        let _ = buffer.insert(5, "5");
        // [5, 6, _, _, _]
        assert!(buffer.consume().is_empty());
        // [5, 6, _, _, _]
        let _ = buffer.insert(4, "4");
        // [5, 6, _, _, 4]
        assert_eq!(buffer.consume(), vec!["4", "5", "6"]);
        // [_, _, _, _, _]
        let _ = buffer.insert(11, "11");
        // [_, 11, _, _, _]
        assert!(buffer.consume().is_empty());
        // [_, 11, _, _, _]
        let _ = buffer.insert(10, "10");
        // [10, 11, _, _, _]
        assert!(buffer.consume().is_empty());
        // [10, 11, _, _, _]
        let _ = buffer.insert(9, "9");
        // [10, 11, _, _, 9]
        assert!(buffer.consume().is_empty());
        // [10, 11, _, _, 9]
        let _ = buffer.insert(8, "8");
        // [10, 11, _, 8, 9]
        assert!(buffer.consume().is_empty());
        // [10, 11, _, 8, 9]
        let _ = buffer.insert(7, "7");
        // [10, 11, 7, 8, 9]
        assert_eq!(buffer.consume(), vec!["7", "8", "9", "10", "11"]);
        // [_, _, _, _, _]
        assert_eq!(buffer.insert(7, "7"), InsertResult::Duplicate);
        // [_, _, _, _, _]
        assert_eq!(buffer.insert(17, "17"), InsertResult::FullBuffer);
        // [_, _, _, _, _]
        let _ = buffer.insert(16, "16");
        // [_, 16, _, _, _]
        assert!(buffer.consume().is_empty());
        // [_, 16, _, _, _]
        let _ = buffer.insert(12, "12");
        // [_, 16, 12, _, _]
        assert_eq!(buffer.consume(), vec!["12"]);
        // [_, 16, _, _, _]
        let _ = buffer.insert(15, "15");
        // [15, 16, _, _, _]
        assert!(buffer.consume().is_empty());
        // [15, 16, _, _, _]
        let _ = buffer.insert(14, "14");
        // [15, 16, _, _, 14]
        assert!(buffer.consume().is_empty());
        // [15, 16, _, _, 14]
        let _ = buffer.insert(13, "13");
        // [15, 16, _, 13, 14]
        assert_eq!(buffer.consume(), vec!["13", "14", "15", "16"]);
        // [_, _, _, _, _]
        assert_eq!(buffer.insert(2, "2"), InsertResult::Duplicate);
        // [_, _, _, _, _]
    }

    #[test]
    fn multiple_inserts() {
        let mut buffer: OrderedBuffer<_, 5> = OrderedBuffer::new();

        let _ = buffer.insert(0, "0");
        let _ = buffer.insert(1, "1");
        let _ = buffer.insert(2, "2");
        let _ = buffer.insert(3, "3");
        let _ = buffer.insert(4, "4");

        assert_eq!(buffer.consume(), vec!["0", "1", "2", "3", "4"]);
    }

    #[test]
    fn multiple_inserts_backwards() {
        let mut buffer: OrderedBuffer<_, 5> = OrderedBuffer::new();

        let _ = buffer.insert(4, "4");
        let _ = buffer.insert(3, "3");
        let _ = buffer.insert(2, "2");
        let _ = buffer.insert(1, "1");
        let _ = buffer.insert(0, "0");

        assert_eq!(buffer.consume(), vec!["0", "1", "2", "3", "4"]);
    }
}
