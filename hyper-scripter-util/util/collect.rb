# [HS_HELP]: Collect scripts in hyper scripter home.
# [HS_HELP]: Scripts not traced will be added, scripts without an actual file will be purged.
# [HS_HELP]:
# [HS_HELP]: USAGE:
# [HS_HELP]:     hs collect

require_relative './common'

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
      name = '.' + name
    else
      # throw err
      throw "what? #{name}?"
    end
  end
  [name, ty]
end

root = HS_ENV.home
directory_tree(root).each do |full_path|
  script = full_path.delete_prefix(root).delete_prefix('/')
  next unless shoud_collect?(script)

  name, ty = extract_name(script)

  begin
    HS_ENV.do_hs("which =#{name} 2>/dev/null", true)
    next
  rescue StandardError
  end

  puts "collecting script #{script}!"

  file = File.open(full_path)
  HS_ENV.do_hs("edit =#{name} -T #{ty} --fast", false)
end

HS_ENV.do_hs('ls --grouping=none --name --plain', true).split(' ').each do |name|
  file = begin
    HS_ENV.do_hs("which =#{name} 2>/dev/null", true).strip
  rescue StandardError
    next
  end

  next if File.exist?(file)

  puts "removing script #{file}!"
  HS_ENV.do_hs("rm --purge =#{name}", true)
end
