# frozen_string_literal: true
#
# [HS_HELP]: Run the selector terminal UI.
# [HS_HELP]:
# [HS_HELP]: e.g.:
# [HS_HELP]:     hs selector opt1 opt2 opt3

require_relative './common'
require 'io/console'

WHITE = "\033[1;37m"
RED = "\033[1;31m"
GREEN = "\033[1;32m"
GREEN_NON_BOLD = "\033[0;32m"
YELLOW = "\033[1;33m"
BLUE = "\033[1;34m"
BLUE_BG = "\033[0;44m"
CYAN = "\033[0;36m"
NC = "\033[0m"

HELP_MSG = "#{GREEN}press h/H for help#{NC}".freeze

def erase_lines(line_count)
  line_count.times do
    $stderr.print "\e[A"
  end
  $stderr.print "\r\e[J"
end

def get_win_width
  IO.console.winsize[1]
end

def compute_lines(len, win_width)
  lines = 1 + len / win_width
  lines -= 1 if (len % win_width).zero?
  lines
end

def compute_search_ranges(s, word)
  return [] if word.empty?

  unless word =~ /[A-Z]/
    s = s.dup
    s.downcase!
  end

  s.to_enum(:scan, word).map do
    start_pos = Regexp.last_match.pre_match.size
    end_pos = start_pos + word.length
    [start_pos, end_pos]
  end
end

def get_containing_range(target, ranges)
  ranges.find { |r| r[0] <= target && r[1] > target }
end

def add_blue_bg(color, needed)
  if needed
    color.sub('m', ';44m')
  else
    color
  end
end


class SelectorResult
  attr_reader :min, :max, :options
  def initialize(min, max, options)
    @min = min
    @max = max
    @options = options
  end
end

class OptionFormatResult
  attr_reader :content, :emphasize
  def initialize(content, emphasize)
    @content = content
    @emphasize = emphasize
  end
end

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
    keys.each { |k| @callbacks.store(k, self.class.make_callback(callback, recur)) }
    @helps.push(self.class.make_help(keys, msg, :no, recur)) unless msg.nil?
  end

  def register_keys_virtual(keys, callback, msg: '', recur: false)
    keys = [keys] unless keys.is_a?(Array)
    should_register_normal = true
    keys.each do |k|
      @virtual_callbacks.store(k, self.class.make_callback(callback, recur))

      # also register single select unless already has been
      should_register_normal = false if @callbacks.include?(k)
    end

    if should_register_normal
      normal_callback = lambda { |pos, opt|
        callback.call(pos, pos + 1, [opt])
      }
      register_keys(keys, normal_callback, msg: nil, recur: recur)
      @helps.push(self.class.make_help(keys, msg, :both, recur))
    else
      @helps.push(self.class.make_help(keys, msg, :yes, recur))
    end
  end

  # Initiate the selector
  def initialize(offset: 1)
    @options = []
    @display_offset = offset
    @search_string = ''
    @number = nil
    @callbacks = {}
    @virtual_callbacks = {}
    @helps = []
    @enter_overriden = false
    @virtual_state = nil
  end

  def can_virtual?
    !@virtual_callbacks.empty?
  end

  def is_virtual_selected(pos)
    @virtual_state.nil? ? false : @virtual_state.in_range?(pos)
  end

  def print_help
    lines = 0
    win_width = get_win_width
    msgs = collect_help_str
    msgs.each do |(msg, len)|
      warn msg
      lines += compute_lines(len, win_width)
    end
    warn '(press any key to continue)'
    lines + 1
  end

  def before_each_render(has_sequence)
  end

  def run(sequence: '')
    pos = 0
    mode = :normal
    win_width = get_win_width
    help_printed = false
    loop do
      if sequence.empty? && !help_printed
        warn HELP_MSG.to_s if sequence.empty?
        help_printed = true
      end

      before_each_render(!sequence.empty?)

      option_count = @options.length
      raise Empty if option_count.zero?

      line_count = 0
      @virtual_state&.set_point(pos)

      if sequence.empty?
        @options.each_with_index do |_, i|
          leading = pos == i ? '>' : ' '
          opt_fmt = format_option(i)
          option = opt_fmt.content
          gen_line = ->(s) { "#{leading} #{i + @display_offset}. #{s}" }
          line_count += compute_lines(gen_line.call(opt_fmt.content).length, win_width) # calculate line height without color, since color will mess up char count
          option = color_line(i, opt_fmt)
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

      resp = if sequence.empty?
               read_char
             else
               ch = sequence[0]
               sequence = sequence[1..]
               ch
             end

      callback = nil

      case mode
      when :search
        case resp
        when "\b", "\c?"
          if @search_string.empty?
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
      when :number
        case resp
        when "\b", "\c?"
          @number /= 10
          mode = :normal if @number.zero?
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
        when 'h', 'H'
          lines = print_help
          read_char
          erase_lines lines
        when 'q', 'Q'
          raise Quit if @virtual_state.nil?

          @virtual_state = nil

        when 'j', 'J', "\e[B"
          pos = (pos + 1) % option_count
        when 'k', 'K', "\e[A"
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
          if resp =~ /[0-9]/
            mode = :number
            @number = resp.to_i
          elsif ENTER == resp && @virtual_state.nil? && !@enter_overriden
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

      erase_lines line_count if callback.nil? || callback.recur

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
      new_opt_len = @options.length
      pos = [new_opt_len - 1, pos].min
      @virtual_state&.truncate_by_length(new_opt_len)
    end
  end

  def exit_virtual
    @virtual_state = nil
  end

  def self.make_result(pos, content)
    SelectorResult.new(pos, pos + 1, [content])
  end

  def self.make_multi_result(min, max, options)
    SelectorResult.new(min, max, options)
  end

  def self.make_callback(cb, recur)
    ret = Struct.new(:cb, :recur)
    ret.new(cb, recur)
  end

  # virtual = :yes, :no, :both
  def self.make_help(keys, msg, virtual, recur)
    ret = Struct.new(:keys, :msg, :virtual, :recur)
    keys = keys.map do |k|
      if k == ENTER
        '<Enter>'
      else
        k
      end
    end
    ret.new(keys, msg, virtual, recur)
  end

  def format_option(pos)
    option = @options[pos]
    if option.is_a? String
      OptionFormatResult.new(@options[pos], [])
    else
      @options[pos].to_fmt_result
    end
  end

  def color_line(pos, opt_fmt)
    option_str = opt_fmt.content
    emphasize = opt_fmt.emphasize
    search_ranges = []
    blue_bg = is_virtual_selected(pos)
    end_color = if is_virtual_selected(pos)
                  BLUE_BG
                else
                  NC
                end

    search_ranges = compute_search_ranges(option_str, @search_string)
    ctrl_chars = []

    search_ranges.each do |r|
      r1, r2 = r
      another_r = get_containing_range(r1, emphasize)
      ctrl_chars.push([r1, RED])

      another_r = get_containing_range(r2, emphasize)
      if another_r.nil?
        ctrl_chars.push([r2, end_color])
      else
        color = another_r[2]
        color = add_blue_bg(color, blue_bg)
        ctrl_chars.push([r2, color])
      end
    end

    emphasize.each do |r|
      r1, r2, color = r
      color = add_blue_bg(color, blue_bg)
      another_r = get_containing_range(r1, search_ranges)
      if another_r.nil?
        ctrl_chars.push([r1, color])
      end

      another_r = get_containing_range(r2, search_ranges)
      if another_r.nil?
        ctrl_chars.push([r2, end_color])
      end
    end

    option_str = option_str.dup unless ctrl_chars.empty?

    ctrl_chars = ctrl_chars.sort_by { |c| -c[0] }
    ctrl_chars.each do |c|
      pos, ctrl = c
      option_str.insert(pos, ctrl)
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
      s = format_option(i).content
      s = s.downcase unless @search_string =~ /[A-Z]/
      return i if s.include?(@search_string)
    end
    nil
  end

  def collect_help_str
    single_help_str = lambda do |plain, h|
      c = lambda do |color|
        if plain
          ''
        else
          color
        end
      end
      s = " * #{c.call(GREEN)}#{h.keys.join('/')}#{c.call(NC)}: #{h.msg}"
      s += " #{c.call(RED)}(ends the selector)#{c.call(NC)}" unless h.recur
      if can_virtual?
        case h.virtual
        when :yes
          s += " #{c.call(BLUE)}(virtual)#{c.call(NC)}"
        when :no
          s += " #{c.call(YELLOW)}(non-virtual)#{c.call(NC)}"
        end
      end
      s
    end

    helps = []
    helps.push(self.class.make_help([ENTER], 'select the option', :no, false)) unless @enter_overriden
    helps.push(self.class.make_help(%w[v V], 'start or quit virtual mode', :both, true)) if can_virtual?
    helps += [
      self.class.make_help(['k', 'K', '<Arrow Up>'], 'move up', :both, true),
      self.class.make_help(['j', 'J', '<Arrow Down>'], 'move down', :both, true),
      self.class.make_help(%w[q Q], 'quit selector or virtual mode', :both, false),
      self.class.make_help(['[0~9]'], 'go to the option at given number', :both, true),
      self.class.make_help(['/'], 'search for string', :both, true),
      self.class.make_help(['n/N'], 'search forwards/search backwards', :both, true)
    ] + @helps
    helps.map do |h|
      plain = single_help_str.call(true, h)
      colored = single_help_str.call(false, h)
      [colored, plain.length]
    end
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

if __FILE__ == $PROGRAM_NAME
  selector = Selector.new
  selector.load(ARGV)

  selector.register_keys_virtual([ENTER], lambda { |_, _, _|
  }, msg: 'Select multiple options')

  answer = []
  result = selector.run
  result.options.each { |opt| puts opt }
end
