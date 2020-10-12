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

root = HS_ENV.home
directory_tree(root).each do |full_path|
  script = full_path.delete_prefix(root).delete_prefix('/')
  next if script.start_with? '.'

  name, ext = script.split('.')
  HS_ENV.do_hs("which =#{name} 2>/dev/null")
  next if $?.success?

  puts "collecting script #{script}!"

  file = File.open(full_path)
  content = Shellwords.escape(file.read)
  File.delete(full_path)
  HS_ENV.do_hs("edit #{name} -c #{ext} --fast #{content} --no-template")
end
