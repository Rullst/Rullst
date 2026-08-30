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

/// A named policy object that centralizes authorization for a user/resource pair.
///
/// Implement this trait for a zero-sized domain policy such as `PostPolicy`, then
/// call `PostPolicy::can_edit(&user, &post)` from controllers or templates. Every
/// operation is denied unless the policy overrides it explicitly.
pub trait Policy<User, Resource> {
    /// Determines whether `user` may view this resource.
    fn can_view(_user: &User, _resource: &Resource) -> bool {
        false
    }

    /// Determines whether `user` may create this resource type.
    fn can_create(_user: &User) -> bool {
        false
    }

    /// Determines whether `user` may edit this resource.
    fn can_edit(_user: &User, _resource: &Resource) -> bool {
        false
    }

    /// Determines whether `user` may delete this resource.
    fn can_delete(_user: &User, _resource: &Resource) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyUser;
    struct DummyResource;

    struct PostPolicy;
    struct User {
        id: u64,
        admin: bool,
    }
    struct Post {
        owner_id: u64,
    }

    impl Gate<DummyResource> for DummyUser {}

    impl Policy<User, Post> for PostPolicy {
        fn can_view(_user: &User, _post: &Post) -> bool {
            true
        }

        fn can_edit(user: &User, post: &Post) -> bool {
            user.admin || user.id == post.owner_id
        }
    }

    #[test]
    fn test_default_gate_methods() {
        let user = DummyUser;
        let resource = DummyResource;

        assert!(!DummyUser::can_view(&user, &resource));
        assert!(!DummyUser::can_create(&user));
        assert!(!DummyUser::can_update(&user, &resource));
        assert!(!DummyUser::can_delete(&user, &resource));
    }

    #[test]
    fn named_policy_is_fail_closed_and_supports_owner_or_role_logic() {
        let owner = User {
            id: 7,
            admin: false,
        };
        let stranger = User {
            id: 9,
            admin: false,
        };
        let admin = User { id: 9, admin: true };
        let post = Post { owner_id: 7 };

        assert!(PostPolicy::can_view(&stranger, &post));
        assert!(PostPolicy::can_edit(&owner, &post));
        assert!(PostPolicy::can_edit(&admin, &post));
        assert!(!PostPolicy::can_edit(&stranger, &post));
        assert!(!PostPolicy::can_create(&owner));
        assert!(!PostPolicy::can_delete(&owner, &post));
    }
}
