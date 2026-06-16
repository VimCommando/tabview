#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatibilityClass {
    Compatible,
    AcceptedBugFix,
    IntentionalEnhancement,
    Regression,
}

impl CompatibilityClass {
    pub fn is_failure(self) -> bool {
        matches!(self, Self::Regression)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comparison<T> {
    pub case: String,
    pub expected: T,
    pub actual: T,
    pub class: CompatibilityClass,
}

impl<T: PartialEq> Comparison<T> {
    pub fn compatible(case: impl Into<String>, expected: T, actual: T) -> Self {
        let class = if expected == actual {
            CompatibilityClass::Compatible
        } else {
            CompatibilityClass::Regression
        };
        Self {
            case: case.into(),
            expected,
            actual,
            class,
        }
    }

    pub fn accepted_change(
        case: impl Into<String>,
        expected: T,
        actual: T,
        class: CompatibilityClass,
    ) -> Self {
        debug_assert!(matches!(
            class,
            CompatibilityClass::AcceptedBugFix | CompatibilityClass::IntentionalEnhancement
        ));
        Self {
            case: case.into(),
            expected,
            actual,
            class,
        }
    }
}
