/// A trait for defining declarative authorization policies (Gates) on resources.
/// This allows you to centralize authorization logic for your models.
/// By default, all operations are denied (`false`).
pub trait Gate<Resource> {
    /// Determines if the given user can view the resource.
    fn can_view(_user: &Self, _resource: &Resource) -> bool {
        false
    }

    /// Determines if the given user can create instances of the resource.
    fn can_create(_user: &Self) -> bool {
        false
    }

    /// Determines if the given user can update the specific resource.
    fn can_update(_user: &Self, _resource: &Resource) -> bool {
        false
    }

    /// Determines if the given user can delete the specific resource.
    fn can_delete(_user: &Self, _resource: &Resource) -> bool {
        false
    }
}
