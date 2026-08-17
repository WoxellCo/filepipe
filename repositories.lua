repositories = {}

repositories["repository-name"] = {
    --path = "/var/www/woxell.co",
    path = ".",
    access_list = {}
}

repositories["repository-name"].access_list["name"] = {
    info = { read = true, write = false },
    content = { read = true, write = true }
}