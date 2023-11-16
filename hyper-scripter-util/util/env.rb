# frozen_string_literal: true

# [HS_HELP]: Interactively manage envs from history.
# [HS_HELP]:
# [HS_HELP]: e.g.:
# [HS_HELP]:     hs env -s hs hs/test --limit 20

require_relative './historian'

Historian.humble_run_id

historian = Historian.new("--display=env #{ARGV.join(' ')}", false)
historian.register_keys_virtual([ENTER], lambda { |_, _, options|
}, msg: 'Apply multiple envs')

clear = false
historian.register_keys(%w[c C], lambda { |_, _|
  clear = true
}, msg: 'Clear the selected env')

result = historian.run()

options = []
if result.is_multi
  options = result.options
else
  options = [result.content]
end

File.open(HS_ENV.env_var(:source), 'w') do |file|
  options.each do |opt|
    case ENV['SHELL'].split('/').last
    when 'fish'
      opt.envs.each do |e|
        if clear
          file.write("set -e #{e[0]}")
        else
          file.write("set -gx #{e[0]} #{e[1]}")
        end
      end
    else
      warn "#{ENV['SHELL']} not supported"
    end
  end
end
