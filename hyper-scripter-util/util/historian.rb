# frozen_string_literal: true

# [HS_HELP]: Interactively run script from history.
# [HS_HELP]:
# [HS_HELP]: e.g.:
# [HS_HELP]:     hs historian -s hs hs/test --limit 20

require 'json'
require_relative './common'
require_relative './selector'

DISPLAY_MODE_MAP = {
  0 => 'all',
  1 => 'args',
  2 => 'env',
}

def get_display_mode_int(mode_str)
  for k, v in DISPLAY_MODE_MAP
    return k if v == mode_str
  end
end

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
    @envs.empty? && @content.empty?
  end

  def cmd_body
    "=#{name}! -- #{content}"
  end

  def clear
    @content = ''
    @envs = []
  end

  def envs_str
    envs.map { |e| "#{e[0]}=#{e[1]}" }.join(' ')
  end

  def envs_str_prefix
    s = envs_str
    unless envs_str.empty?
      s += ' '
    end
    s
  end
end

class Historian < Selector
  attr_reader :script_name

  def scripts_str
    no_humble_str = @no_humble ? '--no-humble': ''
    dir_str = @dir.nil? ? '' : "--dir #{@dir}"
    display_str = "--display=#{DISPLAY_MODE_MAP[@display]}"
    script_str = @scripts.map { |s| "=#{s}!" }.join(' ')
    "#{no_humble_str} #{display_str} #{dir_str} #{script_str}"
  end

  def history_show
    return '' if @scripts.empty?

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
                 ls --grouping=none --plain --format='{{name}}' #{query_str}", false).split
  end

  def initialize(args, register = true)
    @raise_err = false
    arg_obj_str = HS_ENV.do_hs("--dump-args history show #{escape_wildcard(args)}", false)
    arg_obj = JSON.parse(arg_obj_str)

    show_obj = arg_obj['subcmd']['History']['subcmd']['Show']
    @display_mode_bar_printed = false
    @offset = show_obj['offset']
    @limit = show_obj['limit']
    @dir = show_obj['dir']
    @display = get_display_mode_int(show_obj['display'].downcase)
    @no_humble = show_obj['no_humble']
    query = show_obj['queries']
    @single = query.length == 1 && !query[0].include?('*')

    load_scripts(query, arg_obj['root_args'])

    super(offset: @offset + 1)

    load_history
    warn "historian for #{@scripts[0]}" if @single

    register_all if register
  end

  def pos_len(pos)
    Math.log(pos + @offset + 1, 10).floor
  end

  def format_option(pos)
    opt = @options[pos]
    just = @max_name_len - pos_len(pos)
    name = if @single
             ' ' * (just - opt.name.length)
           else
             "(#{opt.name}) ".rjust(just + 3)
           end
    envs_str = opt.envs_str
    envs_str = "(#{envs_str}) " unless envs_str.empty?
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
    run_empty = false
    create = false
    register_keys('.', lambda { |_, _|
      run_empty = true
    }, msg: 'Run script with empty argument')

    register_keys(%w[r R], lambda { |_, obj|
      sourcing = true
      HS_ENV.do_hs("history rm #{scripts_str} -- #{obj.number}", false)
    }, msg: 'Replace the argument')

    register_keys(%w[c C], lambda { |_, _|
      sourcing = true
    }, msg: 'Set next command')

    register_keys_virtual(%w[p P], lambda { |_, _, options|
      options.reverse.each do |opt|
        cmd = "run --no-caution --dummy #{opt.cmd_body}"
        HS_ENV.system_hs(cmd, false, opt.envs)
      end
      load_history
      exit_virtual
    }, msg: 'Push the event to top', recur: true)

    register_keys_virtual([ENTER], lambda { |_, _, _|
    }, msg: 'Run the script')

    register_keys_virtual(%w[a A], lambda { |_, _, _|
      create = true
    }, msg: 'Create new anonymous script')

    result = run(sequence: sequence)

    opt = result.options[0]
    opt.clear if run_empty

    if sourcing
      cmd = "#{opt.envs_str_prefix}#{HS_ENV.env_var(:cmd)} #{opt.cmd_body}"
      commandline(cmd)
    elsif create
      require 'shellwords'
      content = "# [#{"HS_HELP"}]: created from history of `#{opt.name}`\n"
      result.options.each do |opt|
        content += "\n#{opt.envs_str_prefix}#{HS_ENV.env_var(:cmd)} #{opt.cmd_body}"
      end
      content = Shellwords.escape(content)
      HS_ENV.exec_hs("edit --no-template -t +history -- #{content}", false)
    else
      result.options.each do |opt|
        HS_ENV.system_hs(opt.cmd_body, false, opt.envs)
      end
    end
  end

  def get_history
    history = history_show
    opts = []
    cur_number = 0
    history.lines.each do |s|
      s = s.rstrip
      if s.start_with?(' ') # env
        opt = opts[-1]
        next if opt.nil?

        key, _, val = s.strip.partition('=')
        opt.add_env(key, val)
      else
        name, _, content = s.partition(' ')
        opts.push(process_history(name, content, cur_number + @offset + 1))
        cur_number += 1
      end
    end
    opts.reject do |opt|
      if opt.nil?
        true
      elsif @single && opt.empty?
        true
      end
    end
  end

  # User can overwriter this function to create their own option, or apply some filter
  def process_history(name, content, number)
    Option.new(name, content, number)
  end

  def load_history
    load(get_history)
    @max_name_len = @options.each_with_index.map do |opt, i|
      opt.name.length + pos_len(i)
    end.max
  end

  def before_each_render(has_sequence)
    return if has_sequence

    if @display_mode_bar_printed
      erase_lines 1
    end
    @display_mode_bar_printed = true

    display_mode_bar_msg = DISPLAY_MODE_MAP.map do |mode_int, mode_str|
      if mode_int == @display
        "#{RED}#{mode_str}#{WHITE}"
      else
        mode_str
      end
    end.join(' -> ')

    warn "#{WHITE}Display mode: #{display_mode_bar_msg}#{NC}"
  end

  def register_all
    register_keys_virtual(%w[e], lambda { |_, _, _|
      @display = (@display + 1) % DISPLAY_MODE_MAP.length
      load_history
    }, msg: 'toggle show env mode', recur: true)

    register_keys_virtual(%w[E], lambda { |_, _, _|
      @display = (@display - 1) % DISPLAY_MODE_MAP.length
      load_history
    }, msg: 'toggle show env mode (backwards)', recur: true)

    register_keys_virtual(%w[d D], lambda { |_, _, options|
      last_num = nil
      options.each do |opt|
        # TODO: test this and try to make it work
        raise 'Not a continuous range!' unless last_num.nil? || (last_num + 1 == opt.number)

        last_num = opt.number
      end

      min = options[0].number
      max = options[-1].number + 1
      HS_ENV.do_hs("history rm #{scripts_str} -- #{min}..#{max}", false)
      load_history
      exit_virtual
    }, msg: 'delete the history', recur: true)
  end

  # prevent the call to `util/historian` screw up historical query
  # e.g. hs util/historian !
  def self.humble_run_id
    run_id = HS_ENV.env_var(:run_id)
    HS_ENV.do_hs("history humble #{run_id}", false) if run_id != ""
  end

  def self.rm_run_id
    HS_ENV.do_hs("history rm-id #{HS_ENV.env_var(:run_id)}", false)
  end
end

if __FILE__ == $PROGRAM_NAME
  Historian.humble_run_id

  def split_args
    idx = ARGV.find_index('--sequence')
    if idx.nil?
      ['', ARGV.join(' ')]
    else
      seq = ARGV[idx + 1] || ''
      first = ARGV[...idx]
      second = ARGV[(idx + 2)..] || []
      args = "#{first.join(' ')} #{second.join(' ')}"
      seq = seq.gsub("\n", ENTER)
      [seq, args]
    end
  end

  sequence, args = split_args
  historian = Historian.new(args)
  historian.run_as_main(sequence: sequence)
end
