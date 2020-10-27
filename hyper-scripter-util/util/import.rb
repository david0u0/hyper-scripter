# [HS_USAGE]: Import scripts from another hyper scripter home or git repo.
# [HS_USAGE]:
# [HS_USAGE]: USAGE:
# [HS_USAGE]:     hs import [dirname | git repo address]

require 'fileutils'

def copy_unless_exists(src_dir, dst_dir, target)
  src = "#{src_dir}/#{target}"
  dst = "#{dst_dir}/#{target}"
  FileUtils.cp_r src, dst, verbose: true if File.exist?(src) && !File.exist?(dst)
end

if ARGV.length == 0
  puts 'At least one argument is required!'
  exit 1
end

require 'shellwords'
require_relative './common.rb'

class Script
  attr_reader :name, :category, :tags
  def initialize(name, category, tags)
    @name = name
    @category = category
    tags = ['all'] if tags.length == 0
    @tags = tags
  end
end

def parse(ls_string)
  tags = []
  scripts = []
  ret = []
  ls_string.gsub(/(\[|\])/, ' ').split(/[\s\n\r\t]+/).each do |s|
    next if s.length == 0

    if s.start_with?('#')
      if scripts.length != 0
        ret.concat(scripts)
        tags = []
        scripts = []
      end
      tags.push(s[1..-1])
    else
      match = /(?<name>[^(]+)\((?<category>.+)\)/.match(s)
      scripts.push(Script.new(match[:name], match[:category], tags)) unless match.nil?
    end
  end
  ret.concat(scripts)
  ret
end

def import_dir(dir)
  dir = File.expand_path(dir)
  puts "import directory #{dir}"
  out = HS_ENV.do_hs('ls --plain', [], dir)
  parse(out).each do |script|
    HS_ENV.do_hs("which =#{script.name} 2>/dev/null")
    if $?.success?
      puts "#{script.name} already exists!"
      next
    else
      puts "importing #{script.name}..."
      content = HS_ENV.do_hs("cat =#{script.name}", [], dir)
      content = Shellwords.escape(content)
      HS_ENV.do_hs("edit =#{script.name} -c #{script.category} --no-template --fast #{content}")
      tags_str = script.tags.join(',')
      HS_ENV.do_hs("mv =#{script.name} -t #{tags_str}")
    end
  end

  puts 'Copying git directory...'
  copy_unless_exists(dir, HS_ENV.home, '.git')
  puts 'Copying gitignore...'
  copy_unless_exists(dir, HS_ENV.home, '.gitignore')
end

def import(arg)
  if File.directory?(arg)
    import_dir(arg)
  else
    cur = Dir.pwd
    Dir.chdir(DIR)
    `rm .tmp -rf`
    `mkdir .tmp`
    Dir.chdir('.tmp')
    `git clone #{arg} repo`
    exit 1 unless $?.success?
    import_dir('repo')
    Dir.chdir(cur)
  end
end

ARGV.each do |arg|
  import(arg)
end
