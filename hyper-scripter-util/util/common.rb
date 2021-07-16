require 'io/console'

RED = "\033[0;31m".freeze
NC = "\033[0m".freeze
ENTER = "\r".freeze

class HSEnv
  ENV_MAP = { name: 'NAME', cmd: 'HS_CMD', source: 'HS_SOURCE', home: 'HS_HOME', exe: 'HS_EXE' }.freeze

  def initialize(script_dir)
    find_hs_env(script_dir)
    warn "hyper script home = #{@home}, executable = #{@exe}"
    @prefix = ''
  end

  def prefix(pref)
    @prefix = pref
  end

  def find_hs_env(_script_dir)
    @home = env_var(:home)
    @exe = env_var(:exe)
  end

  attr_reader :home, :exe

  def do_hs(arg, all, path = @home)
    cmd = hs_command_str(arg, all, path)
    `#{cmd}`
  end

  def env_var(var_name)
    ENV[ENV_MAP[var_name]]
  end

  def exec_hs(arg, all = true, path = @home)
    cmd = hs_command_str(arg, all, path)
    exec cmd.to_s
  end

  private

  def hs_command_str(arg, all, path)
    access_str = ''
    access_str = '-f all --timeless' if all
    "#{@exe} --no-alias -H #{path} #{access_str} #{@prefix} #{arg}"
  end
end

DIR = File.dirname(__FILE__)
HS_ENV = HSEnv.new(DIR)

# selector
class Selector
  class Empty < StandardError
  end

  class Quit < StandardError
  end

  def load(options)
    @options = options
  end

  # Handle customized keys
  def register_keys(keys, callback, msg = '')
    @enter_overriden = true if keys.include?(ENTER)
    keys = [keys] unless keys.is_a?(Array)
    keys.each { |k| @callbacks.store(k, self.class.make_callback(callback, msg)) }
  end

  # Initiate the selector
  # @param offset [Integer, #read] the first visual number of the candidates
  def initialize(options, offset = 1)
    load(options)
    @search_string = ''
    @number = nil
    @offset = offset
    @callbacks = {}
    @enter_overriden = false
  end

  def run
    pos = 0
    mode = :normal
    loop do
      win_width = IO.console.winsize[1]
      option_count = @options.length
      raise Empty if option_count == 0

      line_count = 0
      display_pos = @offset + pos
      @options.each_with_index do |option, i|
        cur_display_pos = @offset + i
        leading = pos == i ? '>' : ' '
        gen_line = ->(content) { "#{leading} #{cur_display_pos}. #{content}" }
        line_count += compute_lines(gen_line.call(option).length, win_width) # calculate line height without color, since colr will mess up char count
        option = option.gsub(@search_string, "#{RED}#{@search_string}#{NC}") if @search_string.length > 0
        $stderr.print gen_line.call(option) + "\n"
      end

      case mode
      when :search
        $stderr.print "/#{@search_string}"
      when :number
        $stderr.print ":#{@number}"
      end

      resp = ' '
      resp = STDIN.getch
      exit if resp == "\u0003" # Ctrl-C

      callback = nil

      if mode == :search
        case resp
        when "\b", "\c?"
          if @search_string.length == 0
            mode = :normal
          else
            @search_string = @search_string[0..-2]
          end
        when ENTER
          mode = :normal
        else
          @search_string += resp
          new_pos = search_index(pos)
          pos = new_pos unless new_pos.nil?
        end
      elsif mode == :number
        case resp
        when "\b", "\c?"
          if @number == 0
            mode = :normal
          else
            @number /= 10
          end
        when ENTER
          mode = :normal
          pos = [@number - @offset, 0].max
          pos = [pos, option_count - 1].min
        else
          @number = @number * 10 + resp.to_i if resp =~ /[0-9]/
        end
      else
        case resp
        when 'q', 'Q'
          raise Quit
        when 'j', 'J'
          pos = (pos + 1) % option_count
        when 'k', 'K'
          pos = (pos - 1 + option_count) % option_count
        when 'n'
          new_pos = search_index(pos + 1)
          pos = new_pos unless new_pos.nil?
        when 'N'
          new_pos = search_index(pos - 1, true)
          pos = new_pos unless new_pos.nil?
        when '/'
          mode = :search
          @search_string = ''
        else
          resp_to_i = resp.to_i
          if resp =~ /[0-9]/
            mode = :number
            @number = resp.to_i
          elsif (resp == ENTER) && !@enter_overriden
            return self.class.make_result(display_pos, @options[pos])
          else
            @callbacks.each do |key, cur_callback|
              next unless key == resp

              callback = cur_callback.cb
              break
            end
          end
        end
      end

      line_count.times do
        $stderr.print "\e[A"
      end
      $stderr.print "\r\e[J"

      next unless callback

      should_break = callback.call(display_pos, @options[pos])
      return self.class.make_result(display_pos, @options[pos]) if should_break == true

      # for options count change
      new_options = @options.length
      pos = new_options - 1 if pos >= new_options
    end
  end

  def self.make_result(pos, content)
    ret = Struct.new(:pos, :content)
    ret.new(pos, content)
  end

  def self.make_callback(cb, content)
    ret = Struct.new(:cb, :content)
    ret.new(cb, content)
  end

  private

  def search_index(pos, reverse = false)
    len = @options.length
    (0..len).each do |i|
      i = if reverse
            (len + pos - i) % len
          else
            (i + pos) % len
          end
      return i if @options[i].include?(@search_string)
    end
    nil
  end

  def compute_lines(len, win_width)
    lines = 1 + len / win_width
    lines -= 1 if len % win_width == 0
    lines
  end
end
