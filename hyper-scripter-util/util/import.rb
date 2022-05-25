# [HS_HELP]: Import scripts from another hyper scripter home or git repo.
# [HS_HELP]: If a namespace is given, scripts will all be in that namespace.
# [HS_HELP]:
# [HS_HELP]: USAGE:
# [HS_HELP]:     hs import [--namespace namespace] [dirname | git repo address]

require 'optparse'
require 'fileutils'
require 'shellwords'
require_relative './common'

def copy_unless_exists(src_dir, dst_dir, target)
  src = "#{src_dir}/#{target}"
  dst = "#{dst_dir}/#{target}"
  FileUtils.cp_r src, dst, verbose: true if File.exist?(src) && !File.exist?(dst)
end

class Script
  attr_reader :name, :ty, :tags

  def initialize(name, ty, tags)
    @name = name
    @ty = ty
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
      match = /(?<name>[^(]+)\((?<ty>.+)\)/.match(s)
      scripts.push(Script.new(match[:name], match[:ty], tags)) unless match.nil?
    end
  end
  ret.concat(scripts)
  ret
end

def import_dir(dir, namespace)
  other_env = HSEnv.new(File.expand_path(dir))

  puts "import directory #{dir}"
  out = other_env.do_hs('ls --plain', true)
  parse(out).each do |script|
    new_name = if namespace.nil? || script.name.start_with?('.')
                 script.name
               else
                 "#{namespace}/#{script.name}"
               end

    begin
      HS_ENV.do_hs("which =#{new_name} 2>/dev/null", true)
      puts "#{new_name} already exists!"
      next
    rescue StandardError
      puts "importing #{script.name} as #{new_name}..."
      content = begin
        other_env.do_hs("cat =#{script.name}", true)
      rescue StandardError => e
        warn(e)
        next
      end

      content = Shellwords.escape(content)
      tags_str = script.tags.join(',')
      HS_ENV.do_hs("edit =#{new_name} -t #{tags_str} -T #{script.ty} --no-template --fast -- #{content}", false)
    end
  end

  if namespace.nil?
    puts 'Copying git directory...'
    copy_unless_exists(dir, HS_ENV.home, '.git')
    puts 'Copying gitignore...'
    copy_unless_exists(dir, HS_ENV.home, '.gitignore')
  end
end

def import(arg, namespace)
  if File.directory?(arg)
    import_dir(arg, namespace)
  else
    cur = Dir.pwd
    Dir.chdir(DIR)
    `rm .tmp -rf`
    `mkdir .tmp`
    Dir.chdir('.tmp')
    `git clone #{arg} repo`
    exit 1 unless $?.success?
    import_dir('repo', namespace)
    Dir.chdir(cur)
  end
end

namespace = nil
opt = OptionParser.new do |opts|
  opts.on('-n', '--namespace NAMESPACE', 'namespace') do |arg|
    namespace = arg
    warn "import with namespace #{namespace}"
  end
end
opt.parse!

if ARGV.length == 0
  puts 'At least one argument is required!'
  exit 1
end

ARGV.each do |arg|
  import(arg, namespace)
end
