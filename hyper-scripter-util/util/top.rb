# frozen_string_literal: true

# [HS_HELP]: Interactively manage all running hs processes.
# [HS_HELP]:
# [HS_HELP]: e.g.:
# [HS_HELP]:     hs top -s hs hs/test

require_relative './common'
require_relative './selector'

IGNORE_LIST = ['util/top'] # Add script name here to ignore it in the list

def should_ignore(msg)
  IGNORE_LIST.each do |name|
    re = Regexp.compile("^#{name}\\b")
    return true if !(msg =~ re).nil?
  end
  false
end

class Option
  attr_reader :pid, :run_id, :msg
  def initialize(pid, run_id, msg)
    @pid = pid
    @run_id = run_id
    @msg = msg
  end
  def to_s
    "#{@pid} #{@msg}"
  end
end

def wait_for_run_id(action, wait_obj)
  return if action.nil?

  wait_id = wait_obj.map{ |obj| "--id #{obj.run_id}" }.join(' ')
  cmd = "top --wait #{wait_id}"
  if action == :wait
    warn "start waiting!"
    HS_ENV.exec_hs(cmd, false)
  elsif action == :source
    commandline("#{HS_ENV.env_var(:cmd)} --no-alias #{cmd} && ")
  elsif action == :create
    require 'shellwords'
    msg = wait_obj.map { |obj| obj.msg }.join(',')
    content = "# [#{"HS_HELP"}]: created from top #{msg}\n"
    content += "\n#{HS_ENV.env_var(:cmd)} --no-alias #{cmd}"
    content = Shellwords.escape(content)
    HS_ENV.exec_hs("edit --no-template -t +top -- #{content}", false)
  end
end

self_run_id = HS_ENV.env_var(:run_id).to_i
top_msg = HS_ENV.do_hs("top #{escape_wildcard(ARGV.join(' '))}", false)
top_options = top_msg.lines.filter_map do |l|
  arr = l.chop.split
  pid = arr[0].to_i
  run_id = arr[1].to_i
  msg = arr[2..].join(' ')
  if run_id == self_run_id
    nil
  elsif should_ignore(msg)
    nil
  else
    Option.new(pid, run_id, msg)
  end
end
selector = Selector.new
selector.load(top_options)

action = nil

selector.register_keys(%w[p P], lambda { |_, obj|
  system("pstree -plsT #{obj.pid}")
}, msg: 'print the ps tree')

selector.register_keys_virtual([ENTER], lambda { |_, _, options|
}, msg: 'do nothing', recur: true)

selector.register_keys_virtual(%w[a A], lambda { |_, _, options|
  action = :create
}, msg: 'Create new anonymous script')

selector.register_keys_virtual(%w[w W], lambda { |_, _, options|
  action = :wait
}, msg: 'wait for process to end')

selector.register_keys_virtual(%w[c C], lambda { |_, _, options|
  action = :source
}, msg: 'wait for process to end, but in the next commandline')

begin
  wait_obj = selector.run.options
  wait_for_run_id(action, wait_obj)
rescue Selector::Empty
  warn 'No existing process'
  exit
rescue Selector::Quit
  exit
end
