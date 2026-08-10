repositories = {}

repositories["repository-name"] = {
    path = "/var/www/woxell.co",
    access = {}
}

repositories.access["name"] = {
    info = { r = true, w = false },
    content = { r = true, w = true }
}