/// A chip family. Each revision of a family may support different
/// instructions, so get the instruction set from [`ChipSpec::isa`](crate::ChipSpec::isa).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum IsaKind {
    Calco,
}
