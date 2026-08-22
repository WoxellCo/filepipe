users = {
    mk = {
        remote_username = "mk",
        priv_key_path = ".fp/access/key"
    }
}

bindings = {
    media = {
        local_path = "assets/media",
        remote_repository_name = "repository-name",
        remote_address = "https://127.0.0.1:32879",
        default_user = "mk" --string or object, can be both
    },
    publish = {
        local_path = "out",
        remote_repository_name = "woxell.co",
        remote_address = "https://127.0.0.1:32879",
        default_user = "mk" --string or object, can be both
    }
}