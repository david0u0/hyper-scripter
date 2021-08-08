require 'io/console'

RED = "\033[0;31m".freeze
YELLOW_BG = "\033[0;43m".freeze
YELLOW_BG_RED = "\033[31;43m".freeze
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
  def initialize(options)
    load(options)
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
          is_virtual_selected = @virtual_state.nil? ? false : @virtual_state.in_range?(i)
          leading = pos == i ? '>' : ' '
          gen_line = ->(option) { "#{leading} #{i}. #{option}" }
          line_count += compute_lines(gen_line.call(option).length, win_width) # calculate line height without color, since colr will mess up char count
          option = self.class.color_line(option, @search_string, is_virtual_selected)
          option = gen_line.call(option)

          option = "#{YELLOW_BG}#{option}#{NC}" if is_virtual_selected
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
          pos = [@number, 0].max
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

  def self.color_line(option, search_string, is_virtual_selected)
    option = option.to_s
    if is_virtual_selected
      return option.gsub(search_string, "#{YELLOW_BG_RED}#{search_string}#{YELLOW_BG}") if search_string.length > 0
    elsif search_string.length > 0
      return option.gsub(search_string, "#{RED}#{search_string}#{NC}")
    end
    option
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
      return i if @options[i].to_s.include?(@search_string)
    end
    nil
  end

  def compute_lines(len, win_width)
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