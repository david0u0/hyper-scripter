# frozen_string_literal: true

# [HS_HELP]: Interactively manage all running hs processes.
# [HS_HELP]:
# [HS_HELP]: e.g.:
# [HS_HELP]:     hs top -s hs hs/test --limit 20

require_relative './common'
require_relative './selector'
require 'shellwords'

def escape_wildcard(s)
  s.gsub('*', '\*')
end


class Option
  attr_reader :pid, :run_id
  def initialize(pid, run_id, msg)
    @pid = pid
    @run_id = run_id
    @msg = msg
  end
  def to_s
    "#{@pid} #{@msg}"
  end
end

def wait_for_run_id(wait_obj)
  wait_id = wait_obj.map{ |obj| "--id #{obj.run_id}" }.join(' ')
  cmd = "#{HS_ENV.env_var(:cmd)} --no-alias top --wait #{wait_id} && "
  File.open(HS_ENV.env_var(:source), 'w') do |file|
    case ENV['SHELL'].split('/').last
    when 'fish'
      file.write("commandline #{Shellwords.escape(cmd)}")
    else
      warn "#{ENV['SHELL']} not supported"
    end
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
  else
    Option.new(pid, run_id, msg)
  end
end
selector = Selector.new
selector.load(top_options)

selector.register_keys(%w[p P], lambda { |_, obj|
  system("pstree -pT #{obj.pid}")
}, msg: 'print the ps tree')

wait_obj = []
selector.register_keys(%w[w W], lambda { |_, obj|
  wait_obj.push(obj)
}, msg: 'wait for process to end')
selector.register_keys_virtual(%w[w W], lambda { |_, _, options|
  wait_obj = options
}, msg: 'wait for process to end')

begin
  result = selector.run

  unless wait_obj.nil?
    wait_for_run_id(wait_obj) unless wait_obj.empty?
  end
rescue Selector::Empty
  warn 'No existing process'
  exit
rescue Selector::Quit
  exit
end
