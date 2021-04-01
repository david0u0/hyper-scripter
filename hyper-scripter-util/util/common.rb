class HSEnv
  def initialize(script_dir)
    find_hs_env(script_dir)
    warn "hyper script home = #{@hs_home}, executable = #{@hs_exe}"
    @prefix = ''
  end

  def prefix(pref)
    @prefix = pref
  end

  def find_hs_env(script_dir)
    path_script = File.join(script_dir, 'hs_env.sh')
    env = `bash #{path_script}`.delete("\n")
    @hs_home, @hs_exe = env.split(':')
  end

  def home
    @hs_home
  end

  def do_hs(arg, all, path = @hs_home)
    cmd = hs_command_str(arg, all, path)
    `#{cmd}`
  end

  def exec_hs(arg, all = true, path = @hs_home)
    cmd = hs_command_str(arg, all, path)
    exec cmd.to_s
  end

  private

  def hs_command_str(arg, all, path)
    access_str = ''
    access_str = '-f all --timeless' if all
    "#{@hs_exe} --no-alias -H #{path} #{access_str} #{@prefix} #{arg}"
  end
end

DIR = File.dirname(__FILE__)
HS_ENV = HSEnv.new(DIR)
