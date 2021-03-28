# [HS_HELP]: Interactively run script from history.
# [HS_HELP]:
# [HS_HELP]: e.g.:
# [HS_HELP]:     hs historian -f hs hs/test --limit 20

require_relative './common.rb'

lines = []
lines_count = 0
name = ''

load_history = lambda do |args|
  history = HS_ENV.do_hs("history show #{args}", false)
  exit 1 unless $?.success?
  lines = history.lines.map { |s| s.length > 0 ? s : ' ' }
  name = lines.slice!(0).strip

  if lines.length == 0
    puts 'history is empty'
    exit 0
  end
  lines_count = lines.length
end

load_history.call(ARGV.join(' '))

pos = 0
selected = false

loop do
  reload = false
  lines.each_with_index do |line, i|
    leading = i == pos ? '>' : ' '
    $stderr.print "#{leading} #{i + 1}. #{line}"
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
    HS_ENV.do_hs("history rm =#{name}! #{pos + 1}", false)
    load_history.call("=#{name}!")
  end
end

$stderr.print "\e[#{lines_count}E"

if selected
  cmd = "=#{name}! #{lines[pos]}"
  puts cmd
  history = HS_ENV.exec_hs(cmd, false)
end
