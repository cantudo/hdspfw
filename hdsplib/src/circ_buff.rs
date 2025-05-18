pub struct CircBuff<T, const N: usize> {
    buffer: [T; N],
    write_ptr: usize,
    read_ptr: usize,
}

impl<T: Copy + Default, const N: usize> CircBuff<T, N> {
    pub fn new() -> Self {
        Self {
            buffer: [T::default(); N],
            write_ptr: 0,
            read_ptr: 0,
        }
    }

    pub fn push(&mut self, item: T) {
        self.buffer[self.write_ptr] = item;
        self.write_ptr = (self.write_ptr + 1) % N;
        if self.write_ptr == self.read_ptr {
            // Buffer is full, move tail forward
            self.read_ptr = (self.read_ptr + 1) % N;
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.read_ptr == self.write_ptr {
            None // Buffer is empty
        } else {
            let item = self.buffer[self.read_ptr];
            self.read_ptr = (self.read_ptr + 1) % N;
            Some(item)
        }
    }

    pub fn read_exact(&mut self, n: usize, buf: &mut [T]) -> Option<usize> {
        if self.size() < n {
            return None; // Not enough data
        }

        for i in 0..n {
            buf[i] = self.pop().unwrap();
        }
        
        Some(n)
    }

    pub fn size(&self) -> usize {
        if self.write_ptr >= self.read_ptr {
            self.write_ptr - self.read_ptr
        } else {
            N - self.read_ptr + self.write_ptr
        }
    }

    /*
        * Returns the number of elements that can be added to the buffer
        * without overwriting existing data.
    */
    pub fn remaining(&self) -> usize {
        N - self.size() - 1
    }
}