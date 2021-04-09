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

  attr_reader :home

  attr_reader :exe

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
