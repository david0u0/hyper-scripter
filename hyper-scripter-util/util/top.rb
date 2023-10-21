require_relative './common'
require_relative './selector'

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

def wait_for_run_id(run_id)
  while true
    sleep 3
    result = HS_ENV.do_hs("top --id #{run_id}", false).chop
    break if result.empty?
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

wait_obj = nil
selector.register_keys(%w[w W], lambda { |_, obj|
  wait_obj = obj
}, msg: 'wait for process to end')

begin
  result = selector.run

  unless wait_obj.nil?
    warn "Start waiting for #{wait_obj}"
    wait_for_run_id(wait_obj.run_id)
    warn "Process #{wait_obj} ends!"
  end
rescue Selector::Empty
  warn 'No existing process'
  exit
rescue Selector::Quit
  exit
end
