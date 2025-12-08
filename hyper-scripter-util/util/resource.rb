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

def get_script(ls_args)
  if ls_args.length != 0
    query_str = ls_args.map { |s| escape_wildcard(s) }.join(' ')
    lines = HS_ENV.do_hs("ls --grouping=none --plain --format {{id}} #{query_str}", false).lines
    raise "Got multiple scripts: #{lines}" if lines.length != 1
    return lines[0].strip
  end

  hs_top = {}
  HS_ENV.do_hs("top", false).lines do |l|
    pid, _, script_id, _ = l.split(' ')
    hs_top[pid] = script_id
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
  return find_parent_script_id(start, hs_top)
end

SCRIPT = get_script(ls_args)
raise "Can't find script!" if SCRIPT.nil?

warn "Getting resource for script #{SCRIPT}"

DIR = "#{HS_ENV.env_var(:home)}/.resource/#{SCRIPT}"
system("mkdir #{DIR} -p")

edit = false
if rsc_names.length == 0
  require_relative 'selector'

  selector = Selector.new
  selector.load(`ls -t #{DIR}`.lines.map{ |l| l.strip }) # TODO: nested?

  selector.register_keys_virtual(%w[e E], lambda { |_, _, _|
    edit = true
  }, msg: 'Edit the resource file')
  selector.register_keys_virtual(%w[p P], lambda { |_, _, _|
  }, msg: 'Print the resource file path')
  selector.register_keys_virtual([ENTER], lambda { |_, _, options|
  }, msg: 'do nothing', recur: true)

  begin
    rsc_names = selector.run().options
  rescue Selector::Empty
    warn 'No existing resource'
    exit
  rescue Selector::Quit
    exit
  end
end

rsc_files = rsc_names.map { |n| "#{DIR}/#{n}" }

if edit
  exec("vim #{rsc_files.join(' ')}")
else
  rsc_files.each do |n|
    puts n
  end
end
