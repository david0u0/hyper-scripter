require 'io/console'

RED = "\033[0;31m".freeze
YELLOW = "\033[0;33m".freeze
BLUE_BG = "\033[0;44m".freeze
BLUE_BG_RED = "\033[31;44m".freeze
BLUE_BG_YELLOW = "\033[33;44m".freeze
NC = "\033[0m".freeze
ENTER = "\r".freeze

class Selector
  class Empty < StandardError
  end

  class Quit < StandardError
  end

  def load(options)
    @options = options
  end

  # Handle customized keys
  def register_keys(keys, callback, msg: '', recur: false)
    @enter_overriden = true if keys.include?(ENTER)
    keys = [keys] unless keys.is_a?(Array)
    keys.each { |k| @callbacks.store(k, self.class.make_callback(callback, msg, recur)) }
  end

  def register_keys_virtual(keys, callback, msg: '', recur: false)
    keys = [keys] unless keys.is_a?(Array)
    keys.each { |k| @virtual_callbacks.store(k, self.class.make_callback(callback, msg, recur)) }
  end

  # Initiate the selector
  def initialize(offset: 1)
    @options = []
    @display_offset = offset
    @search_string = ''
    @number = nil
    @callbacks = {}
    @virtual_callbacks = {}
    @enter_overriden = false
    @virtual_state = nil
  end

  def can_virtual?
    @virtual_callbacks.length > 0
  end

  def is_virtual_selected(pos)
    @virtual_state.nil? ? false : @virtual_state.in_range?(pos)
  end

  def run(sequence: '')
    pos = 0
    mode = :normal
    loop do
      win_width = IO.console.winsize[1]
      option_count = @options.length
      raise Empty if option_count == 0

      line_count = 0
      @virtual_state&.set_point(pos)

      if sequence.length == 0
        @options.each_with_index do |option, i|
          leading = pos == i ? '>' : ' '
          option = format_option(option)
          gen_line = ->(s) { "#{leading} #{i + @display_offset}. #{s}" }
          line_count += compute_lines(gen_line.call(option), win_width) # calculate line height without color, since colr will mess up char count
          option = color_line(i, option)
          option = gen_line.call(option)

          option = "#{BLUE_BG}#{option}#{NC}" if is_virtual_selected(i)
          $stderr.print("#{option}\n")
        end
      end

      case mode
      when :search
        $stderr.print "/#{@search_string}"
      when :number
        $stderr.print ":#{@number}"
      end

      resp = if sequence.length > 0
               ch = sequence[0]
               sequence = sequence[1..-1]
               ch
             else
               STDIN.getch
             end
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
          @number /= 10
          mode = :normal if @number == 0
        when ENTER
          mode = :normal
          pos = [@number, @display_offset].max
          pos -= @display_offset
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
        when 'v', 'V'
          @virtual_state = (VirtualState.new(pos) if @virtual_state.nil? && can_virtual?)
        else
          resp_to_i = resp.to_i
          if resp =~ /[0-9]/
            mode = :number
            @number = resp.to_i
          elsif (resp == ENTER) && @virtual_state.nil? && !@enter_overriden
            # default enter behavior, for non-virtual mode
            return self.class.make_result(pos, @options[pos])
          else
            callbacks = @virtual_state.nil? ? @callbacks : @virtual_callbacks
            callbacks.each do |key, cur_callback|
              next unless key == resp

              callback = cur_callback
              break
            end
          end
        end
      end

      if callback.nil? || callback.recur
        line_count.times do
          $stderr.print "\e[A"
        end
        $stderr.print "\r\e[J"
      end

      next unless callback

      if @virtual_state.nil?
        callback.cb.call(pos, @options[pos])
        return self.class.make_result(pos, @options[pos]) unless callback.recur
      else
        min, max = @virtual_state.get_range
        opts = @options[min..max - 1]
        callback.cb.call(min, max, opts)
        return self.class.make_multi_result(min, max, opts) unless callback.recur
      end

      # for options count change
      new_options = @options.length
      pos = [@options.length - 1, pos].min
      @virtual_state&.truncate_by_length(@options.length)
    end
  end

  def exit_virtual
    @virtual_state = nil
  end

  def self.make_result(pos, content)
    ret = Struct.new(:is_multi, :pos, :content)
    ret.new(false, pos, content)
  end

  def self.make_multi_result(min, max, options)
    ret = Struct.new(:is_multi, :min, :max, :options)
    ret.new(true, min, max, options)
  end

  def self.make_callback(cb, content, recur)
    ret = Struct.new(:cb, :content, :recur)
    ret.new(cb, content, recur)
  end

  def format_option(opt)
    opt.to_s
  end

  def color_line(pos, option_str)
    if is_virtual_selected(pos)
      if @search_string.length > 0
        return option_str.gsub(@search_string,
                               "#{BLUE_BG_RED}#{@search_string}#{BLUE_BG}")
      end
    elsif @search_string.length > 0
      return option_str.gsub(@search_string, "#{RED}#{@search_string}#{NC}")
    end
    option_str
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
      s = format_option(@options[i])
      return i if s.include?(@search_string)
    end
    nil
  end

  def compute_lines(s, win_width)
    len = s.length
    lines = 1 + len / win_width
    lines -= 1 if len % win_width == 0
    lines
  end
end

class VirtualState
  def initialize(num)
    @fixed = num
    @moving = num
  end

  def set_point(num)
    @moving = num
  end

  def get_range
    if @fixed < @moving
      [@fixed, @moving + 1]
    else
      [@moving, @fixed + 1]
    end
  end

  def truncate_by_length(length)
    @fixed = [length - 1, @fixed].min
    @moving = [length - 1, @moving].min
  end

  def in_range?(num)
    from, to = get_range
    num >= from and num < to
  end
end
