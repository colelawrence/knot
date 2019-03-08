table! {
    services (id) {
        id -> Text,
        display_name -> Text,
        hashsalt -> Text,
        user_id -> Nullable<Text>,
    }
}

table! {
    user_sessions (id) {
        id -> Int4,
        key -> Varchar,
        created_at -> Timestamptz,
        user_token_resource_id -> Nullable<Text>,
        user_id -> Nullable<Text>,
    }
}

table! {
    user_tokens (resource_id) {
        resource_id -> Text,
        access_token -> Text,
        refresh_token -> Text,
        token_expiration -> Timestamptz,
        user_id -> Nullable<Text>,
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

joinable!(services -> users (user_id));
joinable!(user_sessions -> user_tokens (user_token_resource_id));
joinable!(user_sessions -> users (user_id));
joinable!(user_tokens -> users (user_id));

allow_tables_to_appear_in_same_query!(
    services,
    user_sessions,
    user_tokens,
    users,
);
