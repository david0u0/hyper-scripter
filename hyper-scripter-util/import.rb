require 'shellwords'
require_relative './.util.rb'

DIR = File.dirname(__FILE__)
HS_ENV = HSEnv.new(DIR)

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
    content = HS_ENV.do_hs("which =#{script.name} 2>/dev/null")
    if $?.success? && false
      puts "#{script.name} already exist!"
      next
    else
      puts "importing #{script.name}..."
      content = HS_ENV.do_hs("cat =#{script.name}", [], dir)
      content = Shellwords.escape(content)
      HS_ENV.do_hs("edit =#{script.name} -c #{script.category} --no-template -f #{content}")
      tags_str = script.tags.join(',')
      HS_ENV.do_hs("mv =#{script.name} -t #{tags_str}")
    end
  end
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

if ARGV.length == 0
  puts 'at least one argument is required!'
  exit 1
end

ARGV.each do |arg|
  import(arg)
end
