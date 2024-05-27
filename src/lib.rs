use std::{cmp::Ordering, iter::FusedIterator};

#[derive(Debug, PartialEq)]
pub enum InsertResult<'a, T, const N: usize> {
    /// The message was successfully inserted, use the iterator
    /// to receive all messages available so far in order.
    Inserted(OrderedBufferIterator<'a, T, N>),

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
    pub fn insert(&mut self, new_sequence_number: u64, item: T) -> InsertResult<T, N> {
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

                InsertResult::Inserted(OrderedBufferIterator { buffer: self })
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

#[derive(Debug, PartialEq)]
pub struct OrderedBufferIterator<'a, T, const N: usize> {
    buffer: &'a mut OrderedBuffer<T, N>,
}

impl<T, const N: usize> FusedIterator for OrderedBufferIterator<'_, T, N> {}

impl<T, const N: usize> Drop for OrderedBufferIterator<'_, T, N> {
    fn drop(&mut self) {
        // TODO(bschwind) - Is this needed?
        // Exhaust all items in self if they aren't used so that
        // the buffer is returned to a good state.
        for item in self {
            drop(item);
        }
    }
}

impl<T, const N: usize> Iterator for OrderedBufferIterator<'_, T, N> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        // Keep returning items while the item at our read_pos is Some(_).
        self.buffer.items[self.buffer.read_pos].take().map(|(sequence_number, msg)| {
            self.buffer.read_pos = (self.buffer.read_pos + 1) % N;
            self.buffer.next_sequence_number = sequence_number + 1;
            msg
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    trait ToVec<T, const N: usize> {
        fn to_vec(self) -> Vec<T>;
    }

    impl<T: std::fmt::Debug, const N: usize> ToVec<T, N> for InsertResult<'_, T, N> {
        fn to_vec(self) -> std::vec::Vec<T> {
            match self {
                InsertResult::Inserted(iterator) => iterator.collect(),
                _ => {
                    panic!("Expected self to be InsertResult::Inserted, but was actually {self:?}")
                },
            }
        }
    }

    #[test]
    fn it_works() {
        let mut buffer: OrderedBuffer<_, 5> = OrderedBuffer::new();

        assert_eq!(buffer.insert(0, "0").to_vec(), vec!["0"]);
        assert_eq!(buffer.insert(1, "1").to_vec(), vec!["1"]);

        assert!(buffer.insert(3, "3").to_vec().is_empty());
        assert_eq!(buffer.insert(2, "2").to_vec(), vec!["2", "3"]);

        assert!(buffer.insert(6, "6").to_vec().is_empty());
        assert!(buffer.insert(5, "5").to_vec().is_empty());
        assert_eq!(buffer.insert(4, "4").to_vec(), vec!["4", "5", "6"]);

        assert!(buffer.insert(11, "11").to_vec().is_empty());
        assert!(buffer.insert(10, "10").to_vec().is_empty());
        assert!(buffer.insert(9, "9").to_vec().is_empty());
        assert!(buffer.insert(8, "8").to_vec().is_empty());
        assert_eq!(buffer.insert(7, "7").to_vec(), vec!["7", "8", "9", "10", "11"]);

        assert_eq!(buffer.insert(7, "7"), InsertResult::Duplicate);

        assert_eq!(buffer.insert(17, "17"), InsertResult::FullBuffer);

        assert!(buffer.insert(16, "16").to_vec().is_empty());
        assert_eq!(buffer.insert(12, "12").to_vec(), vec!["12"]);
        assert!(buffer.insert(15, "15").to_vec().is_empty());
        assert!(buffer.insert(14, "14").to_vec().is_empty());
        assert_eq!(buffer.insert(13, "13").to_vec(), vec!["13", "14", "15", "16"]);

        assert_eq!(buffer.insert(2, "2"), InsertResult::Duplicate);
    }
}
