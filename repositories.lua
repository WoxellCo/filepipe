repositories = {}

repositories["repository-name"] = {
    --path = "/var/www/woxell.co",
    path = "./dir-u",
    access_list = {}
}

repositories["repository-name"].access_list["mkx"] = {
    info = { read = true, write = false },
    content = { read = true, write = true }
}