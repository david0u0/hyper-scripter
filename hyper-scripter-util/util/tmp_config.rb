# frozen_string_literal: true

# [HS_HELP]: Switch to a temporary hs config based on the current config.

require 'fileutils'
require_relative './common'

cur_config_path = HS_ENV.do_hs("config", false)
tty = `tty`.chop.gsub('/', '_')
tmp_file_path = "/tmp/.hs_config_#{tty}.toml"
FileUtils.cp(cur_config_path, tmp_file_path)
ENV['HYPER_SCRIPTER_CONFIG'] = tmp_file_path

# TODO: add anything you like here, e.g.
# HS_ENV.do_hs("tags set +tmp", false)

exec ENV['SHELL']
