# [HS_HELP]: Interactively run script from history.
# [HS_HELP]:
# [HS_HELP]: e.g.:
# [HS_HELP]:     hs historian -s hs hs/test --limit 20

require 'json'
require_relative './common'
require_relative './selector'

class Option
  def initialize(name, content, number)
    @content = content
    @number = number
    @name = name
  end

  attr_reader :number, :content, :name
end

def escape_wildcard(s)
  s.gsub('*', '\*')
end

class Historian < Selector
  attr_reader :script_name

  def scripts_str
    @scripts.map { |s| "=#{s}!" }.join(' ')
  end

  def history_show
    return '' if @scripts.length == 0

    # when there are multiple scripts, showing humble events will be a mess
    no_humble = @single ? '' : "--no-humble"
    dir_str = @dir.nil? ? '' : "--dir #{@dir}"
    HS_ENV.do_hs(
      "history show --limit #{@limit} --offset #{@offset} \
      --with-name #{dir_str} #{no_humble} #{scripts_str}", false
    )
  end

  def raise_err
    @raise_err = true
  end

  def load_scripts(query, root_args)
    selects = root_args['select']
    timeless = root_args['timeless']
    recent = root_args['recent']
    # TODO: toggle
    # TODO: arch

    select_str = selects.map { |s| "--select #{s}" }.join(' ')
    time_str = if recent.nil?
                 timeless ? '--timeless' : ''
               else
                 "--recent #{recent}"
               end
    query_str = query.map { |s| escape_wildcard(s) }.join(' ')
    @scripts = HS_ENV.do_hs("#{time_str} #{select_str} \
                 ls --grouping none --plain --name #{query_str}", false).split
  end

  def initialize(args)
    @raise_err = false
    arg_obj_str = HS_ENV.do_hs("--dump-args history show #{escape_wildcard(args)}", false)
    arg_obj = JSON.parse(arg_obj_str)

    show_obj = arg_obj['subcmd']['History']['subcmd']['Show']
    @offset = show_obj['offset']
    @limit = show_obj['limit']
    @dir = show_obj['dir'] # TODO: forbid delete?
    query = show_obj['queries']
    @single = query.length == 1 && !query[0].include?('*')

    load_scripts(query, arg_obj['root_args'])

    super(offset: @offset + 1)

    load_history
    warn "historian for #{@options[0]&.name}" if @single
    register_all
  end

  def process_history(name, content, number)
    return nil if (content == '') && @single

    Option.new(name, content, number)
  end

  def pos_len(pos)
    Math.log(pos + @offset + 1, 10).floor
  end

  def format_option(pos)
    opt = @options[pos]
    just = @max_name_len - pos_len(pos)
    if @single
      name = ' ' * (just - opt.name.length)
    else
      name = "(#{opt.name}) ".rjust(just + 3)
    end
    "#{name}#{opt.content}"
  end

  def run(sequence: '')
    if @raise_err
      super(sequence: sequence)
    else
      begin
        super(sequence: sequence)
      rescue Selector::Empty
        warn 'History is empty'
        exit
      rescue Selector::Quit
        exit
      end
    end
  end

  def run_as_main(sequence: '')
    sourcing = false
    echoing = false
    register_keys(%w[p P], lambda { |_, _|
      echoing = true
    }, msg: 'print the argument to stdout')

    register_keys(%w[r R], lambda { |_, obj|
      sourcing = true
      HS_ENV.do_hs("history rm #{scripts_str} #{obj.number}", false)
    }, msg: 'replace the argument')

    register_keys(%w[c C], lambda { |_, _|
      sourcing = true
    }, msg: 'set next command')

    register_keys_virtual([ENTER], lambda { |_, _, options|
    }, msg: 'Run the script multiple times with different arguments')

    result = run(sequence: sequence)

    if result.is_multi
      result.options.each do |opt|
        history = HS_ENV.system_hs("=#{opt.name}! #{opt.content}", false)
      end
      exit
    end

    name = result.content.name
    args = result.content.content
    cmd = "=#{name}! -- #{args}" # known issue: \n \t \" will not be handled properly
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
      HS_ENV.exec_hs(cmd, false)
    end
  end

  def get_history
    history = history_show
    opts = history.lines.each_with_index.map do |s, i|
      s = s.strip
      name, _, content = s.partition(' ')
      process_history(name, content, i + @offset + 1)
    end
    opts.reject { |opt| opt.nil? }
  end

  def load_history
    load(get_history)
    @max_name_len = @options.each_with_index.map do |opt, i|
      opt.name.length + pos_len(i)
    end.max
  end

  def register_all
    register_keys(%w[d D], lambda { |_, obj|
      HS_ENV.do_hs("history rm #{scripts_str} #{obj.number}", false)
      load_history
    }, msg: 'delete the history', recur: true)

    register_keys_virtual(%w[d D], lambda { |_, _, options|
      last_num = nil
      options.each do |opt|
        # TODO: test this and try to make it work
        raise 'Not a continuous range!' unless last_num.nil? || (last_num + 1 == opt.number)

        last_num = opt.number
      end

      min = options[0].number
      max = options[-1].number + 1
      HS_ENV.do_hs("history rm #{scripts_str} #{min}..#{max}", false)
      load_history
      exit_virtual
    }, msg: 'delete the history in range', recur: true)
  end

  # prevent the call to `util/historian` screw up historical query
  # e.g. hs util/historian !
  def self.humble_run_id
    HS_ENV.do_hs("history humble #{HS_ENV.env_var(:run_id)}", false)
  end

  def self.rm_run_id
    HS_ENV.do_hs("history rm-id #{HS_ENV.env_var(:run_id)}", false)
  end
end

if __FILE__ == $0
  Historian.humble_run_id

  def split_args
    if ARGV[0] == '--sequence'
      [ARGV[1], ARGV[2..-1].join(' ')]
    else
      ['', ARGV.join(' ')]
    end
  end

  sequence, args = split_args
  historian = Historian.new(args)
  historian.run_as_main(sequence: sequence)
end
