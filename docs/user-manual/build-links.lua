-- Pandoc filter for build.sh: the HTML goes to generated/, one level below the
-- Markdown, so links between the built pages become .html, and every other
-- relative link or image gets "../" in front.
local built = {
  ["user-manual.md"] = true,
  ["workflow.md"] = true,
  ["doors-to-marreq-migration.md"] = true,
}

local function is_relative(target)
  return not target:match("^%a[%w+.-]*:") and not target:match("^#") and not target:match("^/")
end

local function rewrite(target)
  if not is_relative(target) then
    return target
  end
  local path, fragment = target:match("^([^#]*)(#?.*)$")
  if built[path] then
    return path:gsub("%.md$", ".html") .. fragment
  end
  return "../" .. target
end

function Link(link)
  link.target = rewrite(link.target)
  return link
end

function Image(image)
  image.src = rewrite(image.src)
  return image
end
