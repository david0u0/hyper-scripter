# [HS_HELP]: Interactively run script from history.
# [HS_HELP]:
# [HS_HELP]: e.g.:
# [HS_HELP]:     hs historian -f hs hs/test --limit 20

require 'json'
require_relative './common'
HISTORIAN = ENV['NAME']
ARGS = ARGV.join(' ')

# prevent the call to `util/historian` screw up historical query
# e.g. hs util/historian !
HS_ENV.prefix("--skip-script #{HISTORIAN}")

arg_obj_str = HS_ENV.do_hs("--dump-args history show #{ARGS}", false)
exit 1 unless $?.success?
arg_obj = JSON.parse(arg_obj_str)
filters = arg_obj['filter']
timeless = arg_obj['timeless']
recent = arg_obj['recent']
show_obj = arg_obj['subcmd']['History']['subcmd']['Show']
script_query = show_obj['script']
offset = show_obj['offset']
limit = show_obj['limit']

# ask the actual script by ls command
filter_str = (filters.map { |s| "--filter #{s}" }).join(' ')
time_str = if recent.nil?
             timeless ? '--timeless' : ''
           else
             "--recent #{recent}"
           end
script_name = HS_ENV.do_hs(
  "#{time_str} #{filter_str} ls #{script_query} --grouping none --plain --name",
  false
).strip
exit 1 unless $?.success?

warn "Historian for #{script_name}"

lines = []
lines_count = 0

load_history = lambda do
  history = HS_ENV.do_hs("history show =#{script_name}! --limit #{limit} --offset #{offset}", false)
  exit 1 unless $?.success?
  history.lines.map { |s| s.strip }
end

sourcing = false
selector = Selector.new(load_history.call, offset)
selector.register_keys(%w[d D], lambda { |pos, _|
  HS_ENV.do_hs("history rm =#{script_name}! #{pos}", false)
  selector.load(load_history.call)
})
selector.register_keys(%w[c C], lambda { |_, _|
  sourcing = true
  true
})
args = begin
  selector.run.content
rescue Selector::Empty
  exit
rescue Selector::Quit
  exit
end

cmd = "=#{script_name}! #{args}"
if sourcing
  File.open(ENV['HS_SOURCE'], 'w') do |file|
    case ENV['SHELL'].split('/').last
    when 'fish'
      file.write("commandline \"#{HS_ENV.exe} #{cmd}\"")
    else
      warn "#{ENV['SHELL']} not supported"
    end
  end
else
  warn cmd
  history = HS_ENV.exec_hs(cmd, false)
end
