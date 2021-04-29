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

  def load(lines)
    @lines = lines
  end

  # Handle customized keys
  def register_keys(keys, callback, msg = '')
    keys = [keys] unless keys.is_a?(Array)
    keys.each { |k| @callbacks.store(k, self.class.make_callback(callback, msg)) }
  end

  # Initiate the selector
  # @param offset [Integer, #read] the first visual number of the candidates
  def initialize(lines, offset = 1)
    load(lines)
    @offset = offset
    @callbacks = {}
  end

  def run
    pos = 0
    loop do
      lines_count = @lines.length
      raise Empty if lines_count == 0

      display_pos = @offset + pos
      @lines.each_with_index do |line, i|
        cur_display_pos = @offset + i
        leading = pos == i ? '>' : ' '
        $stderr.print "#{leading} #{cur_display_pos}. #{line}\n"
      end

      resp = ' '
      begin
        system('stty raw -echo')
        resp = $stdin.getc
      ensure
        system('stty -raw echo')
      end

      $stdout.flush

      case resp
      when 'q', 'Q'
        raise Quit
      when 'j', 'J'
        pos = (pos + 1) % lines_count
      when 'k', 'K'
        pos = (pos - 1 + lines_count) % lines_count
      when "\r"
        return self.class.make_result(display_pos, @lines[pos])
      else
        @callbacks.each do |key, callback|
          next unless key == resp

          should_break = callback.cb.call(display_pos, @lines[pos])
          return self.class.make_result(display_pos, @lines[pos]) if should_break == true

          break
        end

        # for lines count change
        new_lines_count = @lines.length
        pos = new_lines_count - 1 if pos >= new_lines_count
      end

      lines_count.times do
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
end
