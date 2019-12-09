#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Id<S> {
    source: S,
    seq: u64,
}

impl<S> Id<S>
where
    S: Copy,
{
    /// Creates a new identifier Id.
    pub fn new(source: S, seq: u64) -> Self {
        Self { source, seq }
    }

    /// Retrieves the source that created this `Id`.
    pub fn source(&self) -> S {
        self.source
    }
}

pub struct IdGen<S> {
    source: S,
    last_seq: u64,
}

impl<S> IdGen<S>
where
    S: Copy,
{
    /// Creates a new generator of `Id`.
    pub fn new(source: S) -> Self {
        Self {
            source,
            last_seq: 0,
        }
    }

    /// Retrives source.
    pub fn source(&self) -> S {
        self.source
    }

    /// Generates the next `Id`.
    pub fn next_id(&mut self) -> Id<S> {
        self.last_seq += 1;
        Id::new(self.source, self.last_seq)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_id() {
        type MyGen = IdGen<u64>;

        // create id generator
        let source = 10;
        let mut gen = MyGen::new(source);

        // generate `n` ids and check the `id` generated
        let n = 100;

        for seq in 1..=n {
            // generate id
            let id = gen.next_id();

            // check `id`
            assert_eq!(id.source, source);
            assert_eq!(id.seq, seq);
        }
    }
}
