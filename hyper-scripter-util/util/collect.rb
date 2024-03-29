# frozen_string_literal: true

# [HS_HELP]: Collect scripts in hyper scripter home.
# [HS_HELP]: Scripts not traced will be added, scripts without an actual file will be purged.
# [HS_HELP]:
# [HS_HELP]: USAGE:
# [HS_HELP]:     hs collect

require_relative './common'

HOME = HS_ENV.home

def directory_tree(path)
  files = []
  Dir.foreach(path) do |entry|
    next if ['..', '.'].include?(entry)

    full_path = File.join(path, entry)
    if File.directory?(full_path)
      directory_tree(full_path).each do |f|
        files.push(f)
      end
    else
      files.push(full_path)
    end
  end
  files
end

def shoud_collect?(file)
  first = true
  file.split('/').each do |path|
    next if first && path == '.anonymous'

    first = false
    return false if path.start_with?('.')
  end
  true
end

def extract_name(file)
  ty = File.extname(file)
  name = file.delete_suffix(ty)
  ty = ty.delete_prefix('.') # the first char is `.`
  ty = 'txt' if ty == ''

  if name.start_with? '.anonymous'
    name = name.sub(%r{^\.anonymous/}, '')
    num = name.to_i
    if num.to_s == name
      name = ".#{name}"
    else
      # throw err
      throw "what? #{name}?"
    end
  end
  [name, ty]
end

existing_files = {}
types = HS_ENV.do_hs("types ls --no-sub", false).split

HS_ENV.do_hs('ls --grouping=none --file --name --plain', true).split.each do |s|
  match = /(?<name>[^(]+)\((?<file>.+)\)/.match(s)
  name = match[:name]
  file = match[:file]
  file = File.join(HOME, file)

  if File.exist?(file)
    existing_files[file] = true
    next
  end

  warn "removing script #{name}!"
  HS_ENV.do_hs("rm --purge =#{name}", true)
end

directory_tree(HOME).each do |full_path|
  script = full_path.delete_prefix(HOME).delete_prefix('/')
  next unless shoud_collect?(script)
  next if existing_files[full_path]

  name, ty = extract_name(script)
  warn "collecting script #{script}!"

  # TODO: handle the case where type name != ext name
  begin
    if types.include?(ty)
      HS_ENV.do_hs("edit =#{name} -T #{ty} --fast", false)
    else
      name = "#{name}.#{ty}"
      warn "try to collect #{name} with type txt"
      HS_ENV.do_hs("edit =#{name} -T txt --fast", false)
    end
  rescue StandardError
  end
end
