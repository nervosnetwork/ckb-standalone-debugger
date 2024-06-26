pub struct HumanReadableCycles(pub u64);

impl std::fmt::Display for HumanReadableCycles {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)?;
        if self.0 >= 1024 * 1024 {
            write!(f, "({:.1}M)", self.0 as f64 / 1024. / 1024.)?;
        } else if self.0 >= 1024 {
            write!(f, "({:.1}K)", self.0 as f64 / 1024.)?;
        } else {
        }
        Ok(())
    }
}
