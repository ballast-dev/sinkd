import platform
from invoke import task

def _openssl_env():
    """
    Dynamically set environment variables based on OS and architecture.
    """
    is_windows = platform.system().lower() == "windows"
    arch_var = platform.machine().lower()

    env = {}
    if is_windows:
        if arch_var in ("amd64", "x86_64"):
            env["OPENSSL_DIR"] = "C:\\Program Files\\OpenSSL-Win64"
            env["OPENSSL_LIB_DIR"] = "C:\\Program Files\\OpenSSL-Win64\\lib\\VC\\x64\\MT"
            env["OPENSSL_INCLUDE_DIR"] = "C:\\Program Files\\OpenSSL-Win64\\include"
        else:
            env["OPENSSL_DIR"] = "C:\\Program Files\\OpenSSL-Win64-ARM"
            env["OPENSSL_LIB_DIR"] = "C:\\Program Files\\OpenSSL-Win64-ARM\\lib\\VC\\arm64\\MT"
            env["OPENSSL_INCLUDE_DIR"] = "C:\\Program Files\\OpenSSL-Win64-ARM\\include"
        env["OPENSSL_STATIC"] = "1"
    # else:
    #     env["OPENSSL_DIR"] = ""
    #     env["OPENSSL_LIB_DIR"] = ""
    #     env["OPENSSL_INCLUDE_DIR"] = ""
    #     env["OPENSSL_STATIC"] = ""

    return env

@task
def clippy(c):
    """
    Run cargo clippy with strict rules.
    """
    c.run(
        "cargo clippy --fix --allow-dirty --allow-staged "
        "-- -W clippy::perf -D clippy::pedantic -D clippy::correctness "
        "-D clippy::suspicious -D clippy::complexity",
        env=_openssl_env(),
    )

# Debug/utility commands
@task
def client(c):
    """
    Run client in debug mode.
    """
    c.run("cargo run -- -d client -s cfg/opt/sinkd/sinkd.conf -u cfg/user/sinkd.conf start", env=_openssl_env())

@task
def client_log(c):
    """
    Tail client log.
    """
    c.run("tail -f /tmp/sinkd/client.log", env=_openssl_env())

@task
def server(c):
    """
    Run server in debug mode.
    """
    c.run("cargo run -- -d server", env=_openssl_env())

@task
def server_log(c):
    """
    Tail server log.
    """
    c.run("tail -f /tmp/sinkd/server.log", env=_openssl_env())

@task
def build(c):
    """
    Build the project with cargo.
    """
    c.run("cargo build", env=_openssl_env())

@task
def clean(c):
    """
    Clean the project with cargo.
    """
    c.run("cargo clean", env=_openssl_env())

@task
def run(c, args=""):
    """
    Run the project with optional args.
    """
    cmd = f"cargo run {args}".strip()
    c.run(cmd, env=_openssl_env())

# ----------------------
# Docker / System tasks
# (Converted from commented lines)
# ----------------------

@task
def usermod_docker(c):
    """
    Add current user to docker group for permissions.
    """
    c.run("sudo usermod -aG docker $(whoami)")

@task
def image(c):
    """
    Create Docker image from Dockerfile.
    """
    c.run("docker build -t alpine -f Dockerfile src/", env=_openssl_env())

@task
def container(c):
    """
    Spawn container with tld mounted as /sinkd.
    """
    c.run("docker run --name sinkd --user $(id -u):$(id -g) -v $(git rev-parse --show-toplevel):/sinkd -itd alpine", env=_openssl_env())

@task
def docker_build(c):
    """
    Build the app in the container.
    """
    c.run("docker exec sinkd cargo build", env=_openssl_env())

@task
def build_no_warn(c):
    """
    Build the app in the container without warnings.
    """
    c.run("docker exec sinkd cargo rustc -- -Awarnings", env=_openssl_env())

@task
def docker_clean(c):
    """
    Clean project inside the container.
    """
    c.run("docker exec sinkd cargo clean", env=_openssl_env())

@task
def rm_container(c):
    """
    Remove (force) the sinkd container.
    """
    c.run("docker container rm -f sinkd", env=_openssl_env())

@task
def rm_image(c):
    """
    Remove (force) the alpine image.
    """
    c.run("docker rmi -f alpine", env=_openssl_env())

@task
def wipe(c):
    """
    Deeper clean: remove container and image.
    """
    rm_container(c)
    rm_image(c)

@task
def attach(c):
    """
    Attach to the running sinkd container.
    """
    c.run("docker container attach sinkd", env=_openssl_env())

@task
def start(c):
    """
    Start Docker service and container.
    """
    c.run("sudo systemctl start docker", env=_openssl_env())
    c.run("docker container start sinkd", env=_openssl_env())

@task
def all(c):
    """
    Example 'all' task: build image, create container, and build.
    """
    image(c)
    container(c)
    docker_build(c)