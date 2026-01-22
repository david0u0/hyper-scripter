# frozen_string_literal: true

ENTER = "\r"

def read_char
  require 'io/console'
  require 'io/wait'
  input = nil
  $stdin.raw do |io|
    io.wait_readable
    input = io.read_nonblock(1)
    if input == "\e"
      begin
        input << io.read_nonblock(3)
      rescue StandardError
        nil
      end
      begin
        input << io.read_nonblock(2)
      rescue StandardError
        nil
      end
    end
  end
  if input == "\n"
    ENTER
  else
    input
  end
ensure
  exit 1 if input == "\u0003" # Ctrl-C
end

def escape_wildcard(s)
  s.gsub('*', '\*')
end

def commandline(args)
  require 'shellwords'
  args = Shellwords.escape(args)
  File.open(HS_ENV.env_var(:source), 'w') do |file|
    case ENV['SHELL'].split('/').last
    when 'fish'
      file.write("commandline #{args}")
    when 'zsh'
      file.write("print -z #{args}")
    else
      warn "#{ENV['SHELL']} not supported"
    end
  end
end

def run_cmd(cmd)
  output = `#{cmd}`
  raise StandardError, "Command `#{cmd}` exit with #{$CHILD_STATUS.exitstatus}" unless $CHILD_STATUS.success?

  output
end

require 'English'
class HSEnv
  ENV_MAP = { name: 'NAME', cmd: 'HS_CMD', run_id: 'HS_RUN_ID', editor: 'HS_EDITOR',
              source: 'HS_SOURCE', home: 'HS_HOME', exe: 'HS_EXE' }.freeze

  def initialize(script_dir = nil)
    find_hs_env(script_dir)
    @prefix = ''
  end

  def prefix(pref)
    @prefix = pref
  end

  attr_reader :home, :exe

  def do_hs(arg, all, envs = [])
    cmd = hs_command_str(arg, all, envs)
    run_cmd(cmd)
  end

  def system_hs(arg, all, envs = [])
    cmd = hs_command_str(arg, all, envs)
    res = system(cmd)
    raise StandardError, 'Hyper scripter exits with error' unless res
  end

  def exec_hs(arg, all, envs = [])
    cmd = hs_command_str(arg, all, envs)
    exec cmd.to_s
  end

  def env_var(var_name)
    k = ENV_MAP[var_name]
    v = ENV[k]
    raise StandardError, "No environment variable #{k} found" if v.nil?

    v
  end

  private

  def find_hs_env(script_dir)
    @home = if script_dir.nil?
              env_var(:home)
            else
              script_dir
            end
    @exe = env_var(:exe)
  end

  def hs_command_str(arg, all, envs = [])
    envs_str = envs.map { |e| "#{e[0]}=#{e[1]}" }.join(' ')
    visible_str = if all
                    '-s all --timeless'
                  else
                    ''
                  end
    "#{envs_str} #{@exe} --no-alias -H #{@home} #{visible_str} #{@prefix} #{arg}"
  end
end

HS_ENV = begin
           HSEnv.new
         rescue => err
           warn "Error initializing hs env: #{err}"
           nil
         end
