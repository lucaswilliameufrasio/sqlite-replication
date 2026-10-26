#define _GNU_SOURCE
#define FUSE_USE_VERSION 31

#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/statvfs.h>
#include <time.h>
#include <unistd.h>

#include <fuse3/fuse.h>

static char backing_root[PATH_MAX];
static char barrier_path[PATH_MAX];
static char release_path[PATH_MAX];
static char completed_path[PATH_MAX];

static int resolve_path(const char *path, char *resolved, size_t size) {
    if (path == NULL || path[0] != '/') {
        return -EINVAL;
    }
    const char *component = path;
    while (*component != '\0') {
        while (*component == '/') {
            component++;
        }
        const char *start = component;
        while (*component != '\0' && *component != '/') {
            component++;
        }
        size_t component_length = (size_t)(component - start);
        if ((component_length == 1 && start[0] == '.') ||
            (component_length == 2 && start[0] == '.' && start[1] == '.')) {
            return -EINVAL;
        }
    }
    int length = snprintf(resolved, size, "%s%s", backing_root, path);
    if (length < 0 || (size_t)length >= size) {
        return -ENAMETOOLONG;
    }
    return 0;
}

static int mark_path(const char *path) {
    int fd = open(path, O_CREAT | O_TRUNC | O_WRONLY | O_CLOEXEC, 0600);
    if (fd < 0) {
        return -errno;
    }
    if (close(fd) != 0) {
        return -errno;
    }
    return 0;
}

static int wait_for_release(void) {
    struct timespec delay = {.tv_sec = 0, .tv_nsec = 10000000};
    while (access(release_path, F_OK) != 0) {
        if (errno != ENOENT) {
            return -errno;
        }
        nanosleep(&delay, NULL);
    }
    return 0;
}

static bool is_capture_stage(const char *path) {
    const char *name = strrchr(path, '/');
    if (name == NULL) {
        name = path;
    } else {
        name++;
    }
    size_t length = strlen(name);
    return strncmp(name, ".capture-", 9) == 0 &&
           length >= 15 &&
           strcmp(name + length - 6, ".stage") == 0;
}

static int passthrough_getattr(const char *path, struct stat *status,
                               struct fuse_file_info *info) {
    if (info != NULL && info->fh != 0) {
        if (fstat((int)(info->fh - 1), status) != 0) {
            return -errno;
        }
        return 0;
    }
    char resolved[PATH_MAX];
    int result = resolve_path(path, resolved, sizeof(resolved));
    if (result != 0) {
        return result;
    }
    if (lstat(resolved, status) != 0) {
        return -errno;
    }
    return 0;
}

static int passthrough_access(const char *path, int mode) {
    char resolved[PATH_MAX];
    int result = resolve_path(path, resolved, sizeof(resolved));
    if (result != 0) {
        return result;
    }
    if (access(resolved, mode) != 0) {
        return -errno;
    }
    return 0;
}

static int passthrough_mkdir(const char *path, mode_t mode) {
    char resolved[PATH_MAX];
    int result = resolve_path(path, resolved, sizeof(resolved));
    if (result != 0) {
        return result;
    }
    if (mkdir(resolved, mode) != 0) {
        return -errno;
    }
    return 0;
}

static int passthrough_unlink(const char *path) {
    char resolved[PATH_MAX];
    int result = resolve_path(path, resolved, sizeof(resolved));
    if (result != 0) {
        return result;
    }
    if (unlink(resolved) != 0) {
        return -errno;
    }
    return 0;
}

static int passthrough_rmdir(const char *path) {
    char resolved[PATH_MAX];
    int result = resolve_path(path, resolved, sizeof(resolved));
    if (result != 0) {
        return result;
    }
    if (rmdir(resolved) != 0) {
        return -errno;
    }
    return 0;
}

static int passthrough_rename(const char *source, const char *destination,
                              unsigned int flags) {
    if (flags != 0) {
        return -EINVAL;
    }
    char resolved_source[PATH_MAX];
    char resolved_destination[PATH_MAX];
    int result = resolve_path(source, resolved_source, sizeof(resolved_source));
    if (result != 0) {
        return result;
    }
    result = resolve_path(destination, resolved_destination,
                          sizeof(resolved_destination));
    if (result != 0) {
        return result;
    }
    if (rename(resolved_source, resolved_destination) != 0) {
        return -errno;
    }
    return 0;
}

static int passthrough_link(const char *source, const char *destination) {
    char resolved_source[PATH_MAX];
    char resolved_destination[PATH_MAX];
    int result = resolve_path(source, resolved_source, sizeof(resolved_source));
    if (result != 0) {
        return result;
    }
    result = resolve_path(destination, resolved_destination,
                          sizeof(resolved_destination));
    if (result != 0) {
        return result;
    }
    if (link(resolved_source, resolved_destination) != 0) {
        return -errno;
    }
    return 0;
}

static int passthrough_open(const char *path, struct fuse_file_info *info) {
    char resolved[PATH_MAX];
    int result = resolve_path(path, resolved, sizeof(resolved));
    if (result != 0) {
        return result;
    }
    int fd = open(resolved, info->flags | O_CLOEXEC);
    if (fd < 0) {
        return -errno;
    }
    info->fh = (uint64_t)fd + 1;
    return 0;
}

static int passthrough_create(const char *path, mode_t mode,
                              struct fuse_file_info *info) {
    char resolved[PATH_MAX];
    int result = resolve_path(path, resolved, sizeof(resolved));
    if (result != 0) {
        return result;
    }
    int fd = open(resolved, info->flags | O_CREAT | O_CLOEXEC, mode);
    if (fd < 0) {
        return -errno;
    }
    info->fh = (uint64_t)fd + 1;
    return 0;
}

static int passthrough_read(const char *path, char *buffer, size_t size,
                            off_t offset, struct fuse_file_info *info) {
    (void)path;
    ssize_t count = pread((int)(info->fh - 1), buffer, size, offset);
    if (count < 0) {
        return -errno;
    }
    return (int)count;
}

static int passthrough_write(const char *path, const char *buffer, size_t size,
                             off_t offset, struct fuse_file_info *info) {
    (void)path;
    ssize_t count = pwrite((int)(info->fh - 1), buffer, size, offset);
    if (count < 0) {
        return -errno;
    }
    return (int)count;
}

static int passthrough_release(const char *path, struct fuse_file_info *info) {
    (void)path;
    if (close((int)(info->fh - 1)) != 0) {
        return -errno;
    }
    return 0;
}

static int passthrough_fsync(const char *path, int data_only,
                             struct fuse_file_info *info) {
    (void)data_only;
    if (is_capture_stage(path)) {
        int result = mark_path(barrier_path);
        if (result != 0) {
            return result;
        }
        result = wait_for_release();
        if (result != 0) {
            return result;
        }
    }
    if (fsync((int)(info->fh - 1)) != 0) {
        return -errno;
    }
    if (is_capture_stage(path)) {
        return mark_path(completed_path);
    }
    return 0;
}

static int passthrough_opendir(const char *path,
                               struct fuse_file_info *info) {
    char resolved[PATH_MAX];
    int result = resolve_path(path, resolved, sizeof(resolved));
    if (result != 0) {
        return result;
    }
    int fd = open(resolved, O_RDONLY | O_DIRECTORY | O_CLOEXEC);
    if (fd < 0) {
        return -errno;
    }
    info->fh = (uint64_t)fd + 1;
    return 0;
}

static int passthrough_readdir(const char *path, void *buffer,
                               fuse_fill_dir_t filler, off_t offset,
                               struct fuse_file_info *info,
                               enum fuse_readdir_flags flags) {
    (void)path;
    (void)offset;
    (void)flags;
    int duplicate = dup((int)(info->fh - 1));
    if (duplicate < 0) {
        return -errno;
    }
    DIR *directory = fdopendir(duplicate);
    if (directory == NULL) {
        int error = errno;
        close(duplicate);
        return -error;
    }
    struct dirent *entry;
    while ((entry = readdir(directory)) != NULL) {
        if (filler(buffer, entry->d_name, NULL, 0, 0) != 0) {
            break;
        }
    }
    closedir(directory);
    return 0;
}

static int passthrough_releasedir(const char *path,
                                  struct fuse_file_info *info) {
    return passthrough_release(path, info);
}

static int passthrough_fsyncdir(const char *path, int data_only,
                                struct fuse_file_info *info) {
    (void)path;
    (void)data_only;
    if (fsync((int)(info->fh - 1)) != 0) {
        return -errno;
    }
    return 0;
}

static int passthrough_truncate(const char *path, off_t size,
                                struct fuse_file_info *info) {
    if (info != NULL && info->fh != 0) {
        if (ftruncate((int)(info->fh - 1), size) != 0) {
            return -errno;
        }
        return 0;
    }
    char resolved[PATH_MAX];
    int result = resolve_path(path, resolved, sizeof(resolved));
    if (result != 0) {
        return result;
    }
    if (truncate(resolved, size) != 0) {
        return -errno;
    }
    return 0;
}

static int passthrough_chmod(const char *path, mode_t mode,
                             struct fuse_file_info *info) {
    (void)info;
    char resolved[PATH_MAX];
    int result = resolve_path(path, resolved, sizeof(resolved));
    if (result != 0) {
        return result;
    }
    if (chmod(resolved, mode) != 0) {
        return -errno;
    }
    return 0;
}

static int passthrough_statfs(const char *path, struct statvfs *status) {
    char resolved[PATH_MAX];
    int result = resolve_path(path, resolved, sizeof(resolved));
    if (result != 0) {
        return result;
    }
    if (statvfs(resolved, status) != 0) {
        return -errno;
    }
    return 0;
}

static struct fuse_operations operations = {
    .getattr = passthrough_getattr,
    .access = passthrough_access,
    .mkdir = passthrough_mkdir,
    .unlink = passthrough_unlink,
    .rmdir = passthrough_rmdir,
    .rename = passthrough_rename,
    .link = passthrough_link,
    .open = passthrough_open,
    .create = passthrough_create,
    .read = passthrough_read,
    .write = passthrough_write,
    .release = passthrough_release,
    .fsync = passthrough_fsync,
    .opendir = passthrough_opendir,
    .readdir = passthrough_readdir,
    .releasedir = passthrough_releasedir,
    .fsyncdir = passthrough_fsyncdir,
    .truncate = passthrough_truncate,
    .chmod = passthrough_chmod,
    .statfs = passthrough_statfs,
};

int main(int argc, char **argv) {
    const char *backing = getenv("CP01_FUSE_BACKING");
    const char *barrier = getenv("CP01_FUSE_BARRIER");
    const char *release = getenv("CP01_FUSE_RELEASE");
    const char *completed = getenv("CP01_FUSE_COMPLETED");
    if (backing == NULL || barrier == NULL || release == NULL ||
        completed == NULL) {
        fprintf(stderr, "CP01 FUSE environment is incomplete\n");
        return 2;
    }
    if (snprintf(backing_root, sizeof(backing_root), "%s", backing) >=
            (int)sizeof(backing_root) ||
        snprintf(barrier_path, sizeof(barrier_path), "%s", barrier) >=
            (int)sizeof(barrier_path) ||
        snprintf(release_path, sizeof(release_path), "%s", release) >=
            (int)sizeof(release_path) ||
        snprintf(completed_path, sizeof(completed_path), "%s", completed) >=
            (int)sizeof(completed_path)) {
        fprintf(stderr, "CP01 FUSE path exceeds PATH_MAX\n");
        return 2;
    }
    return fuse_main(argc, argv, &operations, NULL);
}
