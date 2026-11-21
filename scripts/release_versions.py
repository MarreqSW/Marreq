"""Version registry, changelog and package metadata for Marreq release validation."""

from datetime import date
import json
import re
import tomllib


COMPONENTS = ("server", "cloud", "frontend", "mcp")
BACKENDS = ("server", "cloud")
NPM_DIRS = {"frontend": "frontend", "mcp": "mcp-server"}
VERSION = r"(?:0|[1-9][0-9]{0,15})(?:\.(?:0|[1-9][0-9]{0,15})){2}"
FIELD = re.compile(r"""(?m)^([ \t]*version[ \t]*=[ \t]*)(["'])([^"'\r\n]*)\2(?=[ \t]*(?:#[^\r\n]*)?\r?$)""")
SECTION = re.compile(r"(?m)(?=^\[)")
JSON_SPACE = re.compile(r"[ \t\r\n]*")
METADATA = {
    "Cargo.lock",
    *(f"marreq-{name}/Cargo.toml" for name in BACKENDS),
    *(f"{directory}/{file}" for directory in NPM_DIRS.values()
      for file in ("package.json", "package-lock.json")),
}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def version(value):
    require(isinstance(value, str) and re.fullmatch(VERSION, value),
            f"expected stable SemVer X.Y.Z, got {value!r}")
    fields = tuple(map(int, value.split(".")))
    # npm SemVer represents each numeric field as a JavaScript safe integer.
    require(max(fields) <= 2**53 - 1, f"SemVer field exceeds npm's numeric limit: {value}")
    return fields


def registry(read):
    config = tomllib.loads(read("versions.toml"))
    require(config.keys() == {"marreq", "components"}, "versions.toml: expected marreq and components tables")
    for table, keys in (("marreq", {"version"}), ("components", set(COMPONENTS))):
        require(isinstance(config[table], dict) and config[table].keys() == keys,
                f"versions.toml: [{table}] requires exactly {', '.join(sorted(keys))}")
    values = {"marreq": config["marreq"]["version"], **config["components"]}
    for name, value in values.items():
        try:
            version(value)
        except ValueError as error:
            raise ValueError(f"versions.toml: {name}: {error}") from error
    return values


def changelog(read, name, expected):
    path = f"changelogs/{name}.md"
    headings = {}
    body = []
    current = None
    for line in [*read(path).splitlines(), None]:
        if line is not None and not re.match(r"^[ \t]*##(?:[ \t]|$)", line):
            body.append(line)
            continue
        if current is not None:
            require(any(line.strip() and not line.lstrip().startswith("#") for line in body),
                    f"{path}: [{current}] needs release notes")
        if line is None:
            break
        if line == "## [Unreleased]":
            require(current is None and not headings, f"{path}: duplicate or misplaced Unreleased heading")
            headings["Unreleased"] = None
        else:
            match = re.fullmatch(rf"## \[({VERSION})\] - (Bootstrap|[0-9]{{4}}-[0-9]{{2}}-[0-9]{{2}})", line)
            require(match is not None and "Unreleased" in headings,
                    f"{path}: expected an Unreleased heading and '## [X.Y.Z] - YYYY-MM-DD'")
            number, status = match.groups()
            require(number not in headings, f"{path}: duplicate version {number}")
            if status != "Bootstrap":
                try:
                    date.fromisoformat(status)
                except ValueError as error:
                    raise ValueError(f"{path}: invalid release date {status}") from error
            headings[number] = status
            current = number
        body = []
    entries = list(headings)[1:]
    require(entries and entries[0] == expected, f"{path}: latest entry must match registry version {expected}")
    require(entries == sorted(entries, key=version, reverse=True), f"{path}: versions must descend")
    require(not any(headings[number] == "Bootstrap" for number in entries[:-1]),
            f"{path}: only the oldest version may be marked Bootstrap")
    return {number: headings[number] for number in entries}


def strict_object(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, f"duplicate JSON key {key!r}")
        result[key] = value
    return result


def invalid_constant(value):
    raise ValueError(f"invalid JSON constant {value}")


def member_positions(text, start, wanted):
    """Locate direct member values in an object already validated by json.loads."""
    decoder = json.JSONDecoder()
    cursor = JSON_SPACE.match(text, start + 1).end()
    positions = {}
    while text[cursor] != "}":
        key, cursor = decoder.raw_decode(text, cursor)
        start = JSON_SPACE.match(text, JSON_SPACE.match(text, cursor).end() + 1).end()
        _, end = decoder.raw_decode(text, start)
        if key in wanted:
            positions[key] = (start, end)
            if positions.keys() == wanted:
                break
        cursor = JSON_SPACE.match(text, end).end()
        if text[cursor] == ",":
            cursor = JSON_SPACE.match(text, cursor + 1).end()
    return positions


def npm_updates(path, text, expected):
    data = json.loads(text, object_pairs_hook=strict_object, parse_constant=invalid_constant)
    lock = path.endswith("package-lock.json")
    require(isinstance(data, dict), "expected npm package object")
    if lock:
        require(type(data.get("lockfileVersion")) is int and data["lockfileVersion"] == 3
                and isinstance(data.get("packages"), dict),
                "expected npm lockfile v3")
    name = "marreq-" + path.split("/")[0]
    packages = [data, data["packages"].get("")] if lock else [data]
    for package in packages:
        require(isinstance(package, dict) and package.get("name") == name,
                f"expected metadata for {name}")
        version(package.get("version"))
    if all(package["version"] == expected for package in packages):
        return text
    root = JSON_SPACE.match(text).end()
    top = member_positions(text, root, {"version", "packages"} if lock else {"version"})
    positions = [top["version"]]
    if lock:
        packages_start, _ = top["packages"]
        package_start, _ = member_positions(text, packages_start, {""})[""]
        positions.append(member_positions(text, package_start, {"version"})["version"])
    for (start, end), package in sorted(zip(positions, packages), key=lambda item: item[0], reverse=True):
        if package["version"] != expected:
            text = text[:start] + json.dumps(expected) + text[end:]
    return text


def replace_toml_version(section, old, expected):
    if old == expected:
        return section
    fields = list(FIELD.finditer(section))
    require(len(fields) == 1 and fields[0][3] == old, "expected one plain version assignment")
    start, end = fields[0].span(3)
    return section[:start] + expected + section[end:]


def cargo_updates(path, text, versions):
    data = tomllib.loads(text)
    sections = SECTION.split(text)
    names = {f"marreq-{name}": versions[name] for name in BACKENDS}
    if path != "Cargo.lock":
        name = path.split("/")[0]
        package = data.get("package")
        require(isinstance(package, dict) and package.get("name") == name,
                f"expected [package] metadata for {name}")
        old = package.get("version")
        version(old)
        for index, section in enumerate(sections):
            if section.splitlines() and section.splitlines()[0].strip() == "[package]":
                sections[index] = replace_toml_version(section, old, names[name])
                package["version"] = names[name]
                break
        else:
            raise ValueError("expected a plain [package] header")
    else:
        update_cargo_lock(data, sections, names)

    updated = "".join(sections)
    require(updated == text or tomllib.loads(updated) == data, "cannot safely edit Cargo version fields")
    return updated


def update_cargo_lock(data, sections, names):
    packages = data.get("package")
    require(isinstance(packages, list) and all(isinstance(package, dict) for package in packages),
            "expected Cargo package entries")
    positions = [i for i, section in enumerate(sections) if section.splitlines()[:1] == ["[[package]]"]]
    require(len(packages) == len(positions), "expected standard [[package]] lockfile entries")
    for name, expected in names.items():
        matches = [i for i, package in enumerate(packages) if package.get("name") == name]
        require(len(matches) == 1 and "source" not in packages[matches[0]],
                f"expected exactly one local {name} lockfile entry")
        index = matches[0]
        old = packages[index].get("version")
        version(old)
        position = positions[index]
        sections[position] = replace_toml_version(sections[position], old, expected)
        packages[index]["version"] = expected
    for package in packages:
        dependencies = package.get("dependencies", [])
        require(isinstance(dependencies, list) and all(isinstance(item, str) for item in dependencies),
                f"expected a dependency list for {package.get('name')!r}")
        for dependency in dependencies:
            require(not any(dependency.startswith(name + " ") for name in names),
                    f"unsupported version-qualified workspace lockfile reference {dependency!r}")


def update_metadata(path, text, versions):
    try:
        if path.endswith(".json"):
            component = "frontend" if path.startswith("frontend/") else "mcp"
            return npm_updates(path, text, versions[component])
        return cargo_updates(path, text, versions)
    except (ValueError, TypeError, AttributeError) as error:
        raise ValueError(f"{path}: {error}") from error


def metadata_changes(read, versions):
    changes = {}
    for path in sorted(METADATA):
        original = read(path)
        updated = update_metadata(path, original, versions)
        if updated != original:
            changes[path] = updated
    return changes


def worktree_reader(root):
    def read(path):
        target = root / path
        require(target.resolve(strict=True) == target and target.is_file(),
                f"{path}: expected a regular repository file")
        return target.read_bytes().decode("utf-8")

    return read
