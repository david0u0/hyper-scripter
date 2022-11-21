# [HS_HELP]: Interactively run script from history.
# [HS_HELP]:
# [HS_HELP]: e.g.:
# [HS_HELP]:     hs historian -s hs hs/test --limit 20

require 'json'
require 'shellwords'
require_relative './common'
require_relative './selector'

class Option
  def initialize(name, content, number)
    @content = content
    @number = number
    @name = name
    @envs = []
  end

  attr_reader :number, :content, :name, :envs

  def add_env(key, val)
    @envs.push([key, val])
  end

  def empty?
    @envs.length == 0 && @content.length == 0
  end
  
  def cmd_body
    "=#{name}! -- #{content}"
  end

  def envs_str
    envs.map { |e| "#{e[0]}=#{e[1]}" }.join(' ')
  end
end

def escape_wildcard(s)
  s.gsub('*', '\*')
end

class Historian < Selector
  attr_reader :script_name

  def scripts_str
    # when there are multiple scripts, showing humble events will be a mess
    no_humble = @single ? '' : '--no-humble'
    dir_str = @dir.nil? ? '' : "--dir #{@dir}"
    show_env_str = @show_env ? '--show-env' : ''
    s = @scripts.map { |s| "=#{s}!" }.join(' ')
    "#{no_humble} #{show_env_str} #{dir_str} #{s}"
  end

  def history_show
    return '' if @scripts.length == 0

    HS_ENV.do_hs(
      "history show --limit #{@limit} --offset #{@offset} \
      --with-name #{scripts_str}", false
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
    @dir = show_obj['dir']
    @show_env = show_obj['show_env']
    query = show_obj['queries']
    @single = query.length == 1 && !query[0].include?('*')

    load_scripts(query, arg_obj['root_args'])

    super(offset: @offset + 1)

    load_history
    warn "historian for #{@scripts[0]}" if @single
    register_all
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
    envs_str = opt.envs_str
    envs_str = "(#{envs_str}) " if envs_str.length != 0
    "#{name}#{envs_str}#{opt.content}"
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
      HS_ENV.do_hs("history rm #{} #{scripts_str} #{obj.number}", false)
    }, msg: 'replace the argument')

    register_keys(%w[c C], lambda { |_, _|
      sourcing = true
    }, msg: 'set next command')

    register_keys_virtual([ENTER], lambda { |_, _, options|
    }, msg: 'Run the script multiple times with different arguments')

    result = run(sequence: sequence)

    if result.is_multi
      result.options.each do |opt|
        history = HS_ENV.system_hs(opt.cmd_body, false, opt.envs)
      end
      exit
    end

    opt = result.content
    cmd = opt.cmd_body # known issue: \n \t \" will not be handled properly
    if sourcing
      File.open(HS_ENV.env_var(:source), 'w') do |file|
        case ENV['SHELL'].split('/').last
        when 'fish'
          cmd = "#{opt.envs_str} #{HS_ENV.env_var(:cmd)} #{cmd}"
          file.write("commandline #{Shellwords.escape(cmd)}")
        else
          warn "#{ENV['SHELL']} not supported"
        end
      end
    elsif echoing
      puts opt.content
    else
      HS_ENV.exec_hs(cmd, false, opt.envs)
    end
  end

  def get_history
    history = history_show
    opts = []
    cur_number = 0
    history.lines.each do |s, i|
      s = s.rstrip
      if s.start_with?(' ') # env
        opt = opts[-1]
        key, _, val = s.strip.partition(' ')
        opt.add_env(key, val)
      else
        name, _, content = s.partition(' ')
        opts.push(Option.new(name, content, cur_number + @offset + 1))
        cur_number += 1
      end
    end
    if @single
      opts.reject! { |opt| opt.empty? }
    end
    opts
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
