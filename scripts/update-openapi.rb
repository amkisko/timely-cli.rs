#!/usr/bin/env ruby
# frozen_string_literal: true

require "date"
require "fileutils"
require "json"
require "net/http"
require "uri"

SOURCE_URL = "https://developer.timely.com"
ROOT = File.expand_path("..", __dir__)
DOCS_SPEC_DIR = "tmp/docs/spec"
OPENAPI_JSON_PATH = "tmp/openapi/openapi.json"
API_SPEC_PATH = "tmp/api_spec.md"
WRITE = ARGV.include?("--write") || !ARGV.include?("--check")
MAX_PART_LINES = 260
WRAP = 100

def fetched_date
  index = ARGV.index("--fetched")
  return ARGV.fetch(index + 1) if index

  Date.today.iso8601
end

FETCH_DATE = fetched_date

def render_markdown(spec)
  schemas = spec.dig("components", "schemas") || {}
  md = "# Timely OpenAPI Specification\n\n"
  md += "Source: #{SOURCE_URL}\nFetched: #{FETCH_DATE}\n"
  md += "OpenAPI: #{spec["openapi"]}\nTitle: #{spec.dig("info", "title")}\n"
  md += "Version: #{spec.dig("info", "version")}\n\n"
  md += "## Overview\n\n#{spec.dig("info", "description")}\n\n"
  md += servers_markdown(spec)
  md += security_markdown(spec)
  md += tags_markdown(spec)
  md += "## Operations\n\nTotal operations: #{operations(spec).length}\n\n"
  operations(spec).each { |op| md += operation_markdown(op) }
  md += "## Schemas\n\nTotal schemas: #{schemas.length}\n\n"
  schemas.keys.sort.each do |name|
    md += "### #{name}\n\n```json\n#{JSON.pretty_generate(schemas.fetch(name))}\n```\n\n"
  end
  md
end

def operation_markdown(op)
  md = "### #{op[:method]} #{op[:path]}\n\n"
  md += "- Operation ID: `#{op[:data]["operationId"]}`\n" if op[:data]["operationId"]
  md += "- Summary: #{op[:data]["summary"]}\n" if op[:data]["summary"]
  md += "- Tags: #{op[:data]["tags"].join(", ")}\n" if op[:data]["tags"]
  md += "- Security: `#{JSON.generate(op[:data]["security"])}`\n" if op[:data]["security"]
  md += parameters_markdown(op[:data])
  md += request_body_markdown(op[:data])
  md += responses_markdown(op[:data])
  "#{md}\n"
end

def servers_markdown(spec)
  md = "## Servers\n\n"
  servers = spec["servers"] || []
  if servers.empty?
    return "#{md}- Not declared in document. Default client base URL: https://api.timelyapp.com\n\n"
  end
  servers.each { |server| md += "- #{server["url"]}#{server["description"] ? " - #{server["description"]}" : ""}\n" }
  "#{md}\n"
end

def security_markdown(spec)
  schemes = spec.dig("components", "securitySchemes") || {}
  md = "## Security Schemes\n\n"
  schemes.each do |name, scheme|
    md += "### #{name}\n\n```json\n#{JSON.pretty_generate(scheme)}\n```\n\n"
  end
  md
end

def tags_markdown(spec)
  md = "## Tags\n\n"
  (spec["tags"] || []).each do |tag|
    md += "- **#{tag["name"]}**: #{tag["description"] || ""}\n"
  end
  "#{md}\n"
end

def parameters_markdown(data)
  params = data["parameters"] || []
  return "" if params.empty?

  md = "\nParameters:\n\n| Name | In | Required | Type | Description |\n"
  md += "|---|---:|---:|---|---|\n"
  params.each do |param|
    schema = param["schema"] ? JSON.generate(param["schema"]).tr("|", "\\|") : ""
    desc = (param["description"] || "").gsub("\n", " ").tr("|", "\\|")
    md += "| #{param["name"]} | #{param["in"]} | #{param["required"] ? "yes" : "no"} | "
    md += "#{schema} | #{desc} |\n"
  end
  md
end

def request_body_markdown(data)
  return "" unless data["requestBody"]

  "\nRequest body:\n\n```json\n#{JSON.pretty_generate(data["requestBody"])}\n```\n"
end

def responses_markdown(data)
  responses = data["responses"] || {}
  return "" if responses.empty?

  md = "\nResponses:\n\n"
  responses.each { |code, response| md += "- `#{code}`: #{response["description"] || ""}\n" }
  md
end

def operations(spec)
  spec.fetch("paths", {}).flat_map do |path, item|
    %w[get post put patch delete].map do |method|
      next unless item[method]

      { method: method.upcase, path: path, data: item[method] }
    end
  end.compact.sort_by { |op| op[:data]["operationId"].to_s }
end

def split_wrapped(text, dir, ext)
  lines = text.split("\n", -1).flat_map { |line| wrap(line) }
  lines.each_slice(MAX_PART_LINES).with_index.map do |part_lines, index|
    ["#{dir}/part-#{pad(index)}.#{ext}", "#{part_lines.join("\n")}\n"]
  end
end

def spec_index(count)
  index = "# Timely OpenAPI Specification\n\n"
  index += "Source: #{SOURCE_URL}\nFetched: #{FETCH_DATE}\n\n"
  index += "The complete Markdown rendering is split into small files.\n"
  index += "Read the parts in order.\n\n"
  count.times { |i| index += "- [Part #{pad(i)}](docs/spec/part-#{pad(i)}.md)\n" }
  index
end

def stale_files(expected)
  actual = existing_generated_files
  stale = (actual - expected.keys) + (expected.keys - actual)
  expected.each do |path, content|
    target = File.join(ROOT, path)
    stale << path if File.exist?(target) && File.read(target) != content
  end
  stale.uniq
end

def existing_generated_files
  files = [OPENAPI_JSON_PATH, API_SPEC_PATH]
  spec_dir = File.join(ROOT, DOCS_SPEC_DIR)
  return files unless Dir.exist?(spec_dir)

  Dir.children(spec_dir).sort.each { |name| files << "#{DOCS_SPEC_DIR}/#{name}" }
  files
end

def wrap(text)
  return [""] if text.empty?

  text.scan(/.{1,#{WRAP}}/)
end

def pad(number)
  number.to_s.rjust(3, "0")
end

def fetch_spec
  html = Net::HTTP.get(URI(SOURCE_URL))
  match = html.match(
    %r{<script[^>]*id="api-reference"[^>]*>[\s\S]*?\n\s*(\{[\s\S]*\})\s*\n\s*</script>}
  )
  raise "Could not find Scalar api-reference JSON block" unless match

  spec = JSON.parse(match[1])
  raise "Extracted JSON does not look like OpenAPI" unless spec["openapi"] && spec["paths"]

  spec
end

def generated_files(spec)
  json = "#{JSON.pretty_generate(spec)}\n"
  markdown = render_markdown(spec)
  specs = split_wrapped(markdown, DOCS_SPEC_DIR, "md")
  generated = {
    OPENAPI_JSON_PATH => json,
    API_SPEC_PATH => spec_index(specs.length)
  }
  specs.each { |path, content| generated[path] = content }
  generated
end

def write_generated(files)
  FileUtils.rm_f(File.join(ROOT, "openapi.json"))
  FileUtils.rm_rf(File.join(ROOT, "data"))
  FileUtils.rm_rf(File.join(ROOT, "docs/spec"))
  FileUtils.rm_rf(File.join(ROOT, "tmp/tmp"))
  FileUtils.rm_rf(File.join(ROOT, DOCS_SPEC_DIR))
  FileUtils.rm_rf(File.join(ROOT, "tmp/openapi"))
  FileUtils.rm_f(File.join(ROOT, API_SPEC_PATH))
  files.each do |path, content|
    target = File.join(ROOT, path)
    FileUtils.mkdir_p(File.dirname(target))
    File.write(target, content)
  end
end

spec = fetch_spec
generated = generated_files(spec)

if WRITE
  write_generated(generated)
  puts "Updated Timely OpenAPI: #{operations(spec).length} operations"
else
  stale = stale_files(generated)
  raise "OpenAPI generated files are stale: #{stale.join(", ")}" unless stale.empty?

  puts "Timely OpenAPI generated files are up to date"
end
