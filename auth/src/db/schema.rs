table! {
    user_logins (external_id) {
        external_id -> Text,
        user_id -> Text,
    }
}

table! {
    users (id) {
        id -> Text,
        display_name -> Text,
        full_name -> Nullable<Text>,
        photo_url -> Nullable<Text>,
        is_person -> Bool,
        created_at -> Timestamptz,
    }
}

joinable!(user_logins -> users (user_id));

allow_tables_to_appear_in_same_query!(user_logins, users,);
