# [HS_HELP]: Interactively run script from history.
# [HS_HELP]:
# [HS_HELP]: e.g.:
# [HS_HELP]:     hs historian -f hs hs/test --limit 20

require 'json'
require_relative './common'
require_relative './selector'

NEVER_PROMPT = '--prompt-level never'

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

  def queries_str
    if @script_query.length == 0
      '\*'
    else
      @script_query.map { |q| escape_wildcard(q) }.join(' ')
    end
  end

  def history_show
    prompt_str = @first_load ? '' : NEVER_PROMPT
    @first_load = false
    dir_str = @dir.nil? ? '' : "--dir #{@dir}"
    HS_ENV.do_hs(
      "#{prompt_str} history show #{@root_args_str} \
      --limit #{@limit} --offset #{@offset} \
      --with-name #{dir_str} #{queries_str}", false
    )
  end

  def raise_err
    @raise_err = true
  end

  def initialize(args)
    @raise_err = false
    @first_load = true
    arg_obj_str = HS_ENV.do_hs("--dump-args history show #{escape_wildcard(args)}", false)
    arg_obj = JSON.parse(arg_obj_str)

    show_obj = arg_obj['subcmd']['History']['subcmd']['Show']
    @offset = show_obj['offset']
    @limit = show_obj['limit']
    @dir = show_obj['dir'] # TODO: forbid delete?
    @script_query = show_obj['queries']
    @single = @script_query.length == 1 && !@script_query[0].include?('*')

    root_args = arg_obj['root_args']
    filters = root_args['filter']
    timeless = root_args['timeless']
    recent = root_args['recent']
    # TODO: toggle
    # TODO: arch

    filter_str = (filters.map { |s| "--filter #{s}" }).join(' ')
    time_str = if recent.nil?
                 timeless ? '--timeless' : ''
               else
                 "--recent #{recent}"
               end
    @root_args_str = "#{time_str} #{filter_str}"

    super(offset: @offset + 1)

    load_history
    warn "historian for #{@options[0].name}" if @single && @options.length > 1
    register_all
  end

  def process_history(name, content, number)
    return nil if (content == '') && @single

    Option.new(name, content, number)
  end

  def format_option(opt)
    return opt.content if @single

    name = "(#{opt.name})".ljust(@max_name_len + 2)
    "#{name} #{opt.content}"
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

    register_keys(%w[c C], lambda { |_, _|
      sourcing = true
    }, msg: 'set next command')

    register_keys(%w[r R], lambda { |_, obj|
      sourcing = true
      HS_ENV.do_hs("#{NEVER_PROMPT} history rm #{queries_str} #{obj.number}", false)
    }, msg: 'replce the argument')

    option = run(sequence: sequence).content
    name = option.name
    args = option.content

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
      history = HS_ENV.exec_hs(cmd, false)
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
    @max_name_len = @options.map { |opt| opt.name.length }.max
  end

  def register_all
    register_keys(%w[d D], lambda { |_, obj|
      HS_ENV.do_hs("#{NEVER_PROMPT} history rm #{queries_str} #{obj.number}", false)
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
      HS_ENV.do_hs("#{NEVER_PROMPT} history rm #{queries_str} #{min}..#{max}", false)
      load_history
      exit_virtual
    }, msg: 'delete the history', recur: true)
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
