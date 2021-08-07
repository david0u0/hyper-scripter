# [HS_HELP]: Interactively run script from history.
# [HS_HELP]:
# [HS_HELP]: e.g.:
# [HS_HELP]:     hs historian -f hs hs/test --limit 20

require 'json'
require_relative './common'
require_relative './selector'

class Option
  def initialize(content, number)
    @content = content.strip
    @number = number
  end

  def to_s
    @content
  end
  attr_reader :number, :content
end

class Historian < Selector
  attr_reader :script_name
  def initialize(args)
    arg_obj_str = HS_ENV.do_hs("--dump-args history show #{args}", false)
    arg_obj = JSON.parse(arg_obj_str)

    show_obj = arg_obj['subcmd']['History']['subcmd']['Show']
    @offset = show_obj['offset']
    @limit = show_obj['limit']

    filters = arg_obj['filter']
    timeless = arg_obj['timeless']
    recent = arg_obj['recent']
    script_query = show_obj['script']

    # ask the actual script by ls command
    filter_str = (filters.map { |s| "--filter #{s}" }).join(' ')
    time_str = if recent.nil?
                 timeless ? '--timeless' : ''
               else
                 "--recent #{recent}"
               end
    @script_name = HS_ENV.do_hs(
      "#{time_str} #{filter_str} ls #{script_query} --grouping none --plain --name",
      false
    ).strip

    warn "historian for #{@script_name}"

    super(get_options)

    register_all
  end

  def run(sequence: '')
    res = begin
      super(sequence: sequence)
    rescue Selector::Empty
      puts 'History is empty'
      exit
    rescue Selector::Quit
      exit
    end
    res
  end

  def run_as_main(sequence: '')
    sourcing = false
    echoing = false
    register_keys(%w[p P], lambda { |_, _|
      echoing = true
    }, msg: 'print the argument to stdout')

    register_keys(%w[c C], lambda { |_, _|
      sourcing = true
    }, msg: 'set next command')

    register_keys(%w[r R], lambda { |_, obj|
      sourcing = true
      HS_ENV.do_hs("history rm =#{@script_name}! #{obj.number}", false)
    }, msg: 'replce the argument')

    args = run(sequence: sequence).content.content

    cmd = "=#{@script_name}! #{args}" # known issue: \n \t \" will not be handled properly
    if sourcing
      File.open(HS_ENV.env_var(:source), 'w') do |file|
        case ENV['SHELL'].split('/').last
        when 'fish'
          cmd = "#{HS_ENV.env_var(:cmd)} #{cmd}"
          file.write("commandline #{cmd.inspect}")
        else
          warn "#{ENV['SHELL']} not supported"
        end
      end
    elsif echoing
      puts args
    else
      warn cmd
      history = HS_ENV.exec_hs(cmd, false)
    end
  end

  def get_options
    history = HS_ENV.do_hs("history show =#{@script_name}! --limit #{@limit} --offset #{@offset}", false)
    history.lines.each_with_index.map { |s, i| Option.new(s, i + @offset + 1) }
  end

  def load_options
    load(get_options)
  end

  def register_all
    register_keys(%w[d D], lambda { |_, obj|
      HS_ENV.do_hs("history rm =#{@script_name}! #{obj.number}", false)
      load_options
    }, msg: 'delete the history', recur: true)

    register_keys_virtual(%w[d D], lambda { |min, max, _|
      # FIXME: obj.number?
      HS_ENV.do_hs("history rm =#{@script_name}! #{min + @offset + 1}..#{max + @offset + 1}", false)
      load_options
      exit_virtual
    }, msg: 'delete the history', recur: true)
  end
end

if __FILE__ == $0
  # prevent the call to `util/historian` screw up historical query
  # e.g. hs util/historian !
  HS_ENV.do_hs("history rm-id #{HS_ENV.env_var(:run_id)}", false)

  def split_args(args)
    index = args.index('--')
    if index.nil?
      ['', args.join(' ')]
    else
      [args[0..index].join(' '), args[index + 1..-1].join(' ')]
    end
  end
  sequence, args = split_args(ARGV)

  historian = Historian.new(args)
  historian.run_as_main(sequence: sequence)
end
