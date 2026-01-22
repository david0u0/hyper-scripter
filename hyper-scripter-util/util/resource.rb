#!/usr/bin/env ruby

# frozen_string_literal: true

# [HS_HELP]: Locate the resource files of a script
# [HS_HELP]:
# [HS_HELP]: e.g.:
# [HS_HELP]:     hs util/resource -s hs hs/test -- resource_name
# [HS_HELP]: or:
# [HS_HELP]:     hs util/resource -- resource_name
# [HS_HELP]:
# [HS_HELP]: In the later case, user doesn't provide the script, so this utility will automatically find the script that calls it, if any.
# [HS_HELP]: If the resource name isn't provieded, it will launch a selector showing all existing resource.

require_relative './common'

BASE = "#{HS_ENV.env_var(:home)}/.resource"

class Script
  attr_reader :id, :name
  def initialize(id, name)
    @id = id
    @name = name
  end
end

class Option
  attr_reader :script, :resource_name
  def initialize(script, resource_name)
    @script = script
    @resource_name = resource_name
  end
  def get_path()
    "#{get_base_path}/#{@resource_name}"
  end
  def get_base_path()
    "#{BASE}/#{@script.id}"
  end
end

def split_args
  idx = ARGV.find_index('--')
  if idx.nil?
    [ARGV, []]
  else
    seq = ARGV[idx + 1] || ''
    first = ARGV[...idx]
    second = ARGV[(idx + 2)..] || []
    args = "#{first.join(' ')} #{second.join(' ')}"
    [seq, args]
    [ARGV[...idx], ARGV[idx+1..]]
  end
end

ls_args, rsc_names = split_args

def get_scripts_from_ls_args(ls_args)
  query_str = ls_args.map { |s| escape_wildcard(s) }.join(' ')
  lines = HS_ENV.do_hs("ls --grouping=none --plain --format '{{id}} {{name}}' #{query_str}", false).lines
  lines.map do |l|
    id, name = l.split
    Script.new(id.to_i, name)
  end
end

def get_scripts(ls_args)
  if ls_args.length != 0
    return get_scripts_from_ls_args(ls_args)
  end

  hs_top = {}
  HS_ENV.do_hs("top", false).lines do |l|
    pid, _, script_id, script_name = l.split(' ')
    hs_top[pid] = Script.new(script_id, script_name)
  end

  def get_parent_pid(pid)
    pid = `ps -o ppid= #{pid}`.strip
    pid
  end

  def find_parent_script_id(pid, hs_top)
    return nil if pid == "0"

    return hs_top[pid] unless hs_top[pid].nil?
    pid = get_parent_pid(pid)
    find_parent_script_id(pid, hs_top)
  end

  start = get_parent_pid(get_parent_pid(Process.pid)) # use grand-parent in case it's called with `hs`
  script = find_parent_script_id(start, hs_top)
  unless script.nil?
    return [script]
  end

  warn "Can't find script with top. List all resources for active scripts"
  get_scripts_from_ls_args([])
end

SCRIPTS = get_scripts(ls_args)
raise "Can't find script!" if SCRIPTS.empty?
SINGLE = SCRIPTS.length == 1

edit = false
if rsc_names.length > 0
  raise "Should have exactly one script, got #{SCRIPTS.length}" unless SINGLE
  options = rsc_names.map do |rsc_name|
    Option.new(SCRIPTS[0], rsc_name)
  end
else
  require_relative 'selector'
  class ResourceSelector < Selector
    def load_resources()
      options = []
      SCRIPTS.each do |script|
        dir = "#{BASE}/#{script.id}"
        next unless Dir.exist?(dir)
        ls_res = `ls -t #{dir}` # TODO: nested?
        ls_res.lines do |l|
          options.push(Option.new(script, l.strip))
        end
      end
      load(options)

      @max_name_len = @options.each_with_index.map do |opt, i|
        opt.script.name.length + pos_len(i)
      end.max
    end
    def format_option(pos)
      emphasize = []
      opt = @options[pos]
      just = @max_name_len - pos_len(pos)
      ret = "#{opt.script.name} ".rjust(just + 1)
      len = ret.length
      emphasize.push([len - 1 - opt.script.name.length, len - 1, WHITE])
      OptionFormatResult.new(ret + opt.resource_name, emphasize)
    end
  end

  selector = ResourceSelector.new
  selector.load_resources

  selector.register_keys_virtual(%w[e E], lambda { |_, _, _|
    edit = true
  }, msg: 'Edit the resource file')
  selector.register_keys_virtual(%w[p P], lambda { |_, _, _|
  }, msg: 'Print the resource file path')
  selector.register_keys_virtual([ENTER], lambda { |_, _, options|
  }, msg: 'do nothing', recur: true)

  begin
    options = selector.run().options
  rescue Selector::Empty
    warn 'No existing resource'
    exit
  rescue Selector::Quit
    exit
  end
end

if edit
  exec("#{HS_ENV.env_var(:editor)} #{options.map { |opt| opt.get_path } .join(' ')}")
else
  options.each do |opt|
    system("mkdir #{opt.get_base_path} -p")
    puts opt.get_path
  end
end
