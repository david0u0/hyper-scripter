class HSEnv
  ENV_MAP = { name: 'NAME', cmd: 'HS_CMD', run_id: 'HS_RUN_ID',
              source: 'HS_SOURCE', home: 'HS_HOME', exe: 'HS_EXE' }.freeze

  def initialize(script_dir = nil)
    find_hs_env(script_dir)
    warn "hyper script home = #{@home}, executable = #{@exe}"
    @prefix = ''
  end

  def prefix(pref)
    @prefix = pref
  end

  attr_reader :home, :exe

  def do_hs(arg, all)
    cmd = hs_command_str(arg, all)
    output = `#{cmd}`
    raise StandardError, "Hyper scripter exits with #{$?.exitstatus}" unless $?.success?

    output
  end

  def exec_hs(arg, all)
    cmd = hs_command_str(arg, all)
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

  def hs_command_str(arg, all)
    visible_str = if all
                    '-f all --timeless'
                  else
                    ''
                  end
    "#{@exe} --no-alias -H #{@home} #{visible_str} #{@prefix} #{arg}"
  end
end

HS_ENV = HSEnv.new
