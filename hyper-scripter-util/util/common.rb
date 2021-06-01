require 'io/console'

RED = "\033[0;31m".freeze
NC = "\033[0m".freeze

class HSEnv
  def initialize(script_dir)
    find_hs_env(script_dir)
    warn "hyper script home = #{@home}, executable = #{@exe}"
    @prefix = ''
  end

  def prefix(pref)
    @prefix = pref
  end

  def find_hs_env(script_dir)
    path_script = File.join(script_dir, 'hs_env.sh')
    env = `bash #{path_script}`.delete("\n")
    @home, @exe = env.split(':')
  end

  attr_reader :home, :exe

  def do_hs(arg, all, path = @home)
    cmd = hs_command_str(arg, all, path)
    `#{cmd}`
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
    keys = [keys] unless keys.is_a?(Array)
    keys.each { |k| @callbacks.store(k, self.class.make_callback(callback, msg)) }
  end

  # Initiate the selector
  # @param offset [Integer, #read] the first visual number of the candidates
  def initialize(options, offset = 1)
    load(options)
    @search_string = ''
    @offset = offset
    @callbacks = {}
  end

  def run
    pos = 0
    search_mode = false
    loop do
      win_width = IO.console.winsize[1]
      option_count = @options.length
      line_count = 0
      raise Empty if option_count == 0

      display_pos = @offset + pos
      @options.each_with_index do |option, i|
        cur_display_pos = @offset + i
        leading = pos == i ? '>' : ' '
        gen_line = ->(content) { "#{leading} #{cur_display_pos}. #{content}\n" }
        line_count += gen_line.call(option).length / win_width # calculate line height without color, since colr will mess up char count
        option = option.gsub(@search_string, "#{RED}#{@search_string}#{NC}") if @search_string.length > 0
        $stderr.print gen_line.call(option)
      end
      $stderr.print "/#{@search_string}" if search_mode

      resp = ' '
      resp = STDIN.getch
      exit if resp == "\u0003" # Ctrl-C

      if search_mode
        case resp
        when "\b", "\c?"
          if @search_string.length == 0
            search_mode = false
          else
            @search_string = @search_string[0..-2]
          end
        when "\r"
          search_mode = false
        else
          @search_string += resp
          new_pos = search_index(pos)
          pos = new_pos unless new_pos.nil?
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
        when "\r"
          return self.class.make_result(display_pos, @options[pos])
        when '/'
          search_mode = true
          @search_string = ''
        else
          @callbacks.each do |key, callback|
            next unless key == resp

            should_break = callback.cb.call(display_pos, @options[pos])
            return self.class.make_result(display_pos, @options[pos]) if should_break == true

            break
          end

          # for options count change
          new_options = @options.length
          pos = new_options - 1 if pos >= new_options
        end
      end

      option_count.times do
        $stderr.print "\e[A"
      end
      $stderr.print "\r\e[J"
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
end
