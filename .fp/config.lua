users = {
    mk = {
        remote_username = "mk",
        priv_key_path = "test/test1"
    }
}

bindings = {
    media = {
        local_path = "assets/media",
        remote_repository_name = "repository-name",
        remote_address = "http://127.0.0.1:32879",
        default_user = "mk" --string or object, can be both
    },
    deploy = {
        local_path = "out",
        remote_repository_name = "woxell.co",
        remote_address = "http://127.0.0.1:32879",
        default_user = "mk" --string or object, can be both
    }
}