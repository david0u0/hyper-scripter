# [HS_HELP]: Collect scripts in hyper scripter home.
# [HS_HELP]: Scripts not traced will be added, scripts without an actual file will be purged.
# [HS_HELP]:
# [HS_HELP]: USAGE:
# [HS_HELP]:     hs collect

require_relative './common.rb'
require 'shellwords'

def directory_tree(path)
  files = []
  Dir.foreach(path) do |entry|
    next if entry == '..' || entry == '.'

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
  file.split('/').each do |path|
    return false if path.start_with?('.')
  end
  true
end

root = HS_ENV.home
directory_tree(root).each do |full_path|
  script = full_path.delete_prefix(root).delete_prefix('/')
  next unless shoud_collect?(script)

  name, _, ext = script.rpartition('.')

  HS_ENV.do_hs("which =#{name} 2>/dev/null")
  next if $?.success?

  puts "collecting script #{script}!"

  file = File.open(full_path)
  content = Shellwords.escape(file.read)
  File.delete(full_path)
  HS_ENV.do_hs("edit =#{name} -c #{ext} --fast #{content} --no-template")
end

HS_ENV.do_hs('ls --grouping=none --name --plain').split(' ').each do |name|
  file = HS_ENV.do_hs("which =#{name} 2>/dev/null").delete_suffix("\n")
  next unless $?.success?

  unless File.exist?(file)
    puts "removing script #{file}!"
    HS_ENV.do_hs("rm --purge =#{name}")
  end
end
