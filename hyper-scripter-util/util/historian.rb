# [HS_HELP]: Interactively run script from history.
# [HS_HELP]:
# [HS_HELP]: e.g.:
# [HS_HELP]:     hs historian -f hs hs/test --limit 20

require 'json'
require_relative './common.rb'
HISTORIAN = 'util/historian'.freeze
ARGS = ARGV.join(' ')

# prevent the call to `util/historian` screw up historical query
# e.g. hs util/historian !
HS_ENV.prefix("--skip-script #{HISTORIAN}")

arg_obj_str = HS_ENV.do_hs("--dump-args history show #{ARGS}", false)
exit 1 unless $?.success?
arg_obj = JSON.parse(arg_obj_str)
show_obj = arg_obj['subcmd']['History']['subcmd']['Show']
script = show_obj['script']
offset = show_obj['offset']

lines = []
lines_count = 0

load_history = lambda do
  history = HS_ENV.do_hs("history show #{ARGS}", false)
  exit 1 unless $?.success?
  lines = history.lines.map { |s| s.length > 0 ? s : ' ' }

  lines_count = lines.length
  if lines_count == 0
    $stderr.print 'history is empty'
    exit 0
  end
end

load_history.call

pos = 0
selected = false

loop do
  display_pos = offset + pos + 1
  reload = false
  lines.each_with_index do |line, i|
    cur_display_pos = offset + i + 1
    leading = cur_display_pos == display_pos ? '>' : ' '
    $stderr.print "#{leading} #{cur_display_pos}. #{line}"
  end

  begin
    system('stty raw -echo')
    resp = $stdin.getc

    case resp
    when 'q', 'Q'
      break
    when 'j', 'J'
      pos = (pos + 1) % lines_count
    when 'k', 'K'
      pos = (pos - 1 + lines_count) % lines_count
    when "\r"
      selected = true
      break
    when 'd', 'D'
      reload = true
    end
  ensure
    system('stty -raw echo')
  end

  $stdout.flush

  lines_count.times do
    $stderr.print "\e[A"
  end
  $stderr.print "\r\e[J"

  if reload
    HS_ENV.do_hs("history rm #{script} #{display_pos}", false)
    load_history.call
  end
end

$stderr.print "\e[#{lines_count}E"

if selected
  cmd = "#{script} #{lines[pos]}"
  $stderr.print cmd
  history = HS_ENV.exec_hs(cmd, false)
end
