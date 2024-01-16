# frozen_string_literal: true

require 'bcrypt'

class User < ApplicationRecord
  include BCrypt

  attr_reader :password

  validates :username, presence: true, uniqueness: true, length: { maximum: 32 }
  validates :password, confirmation: true, presence: true, length: { maximum: 16 }, if: :pass_req?
  validates :password_confirmation, presence: true, if: :pass_req?
  validates :p_salt, length: { maximum: 80 }
  validates :p_hash, length: { maximum: 80 }

  before_save :downcase_username

  def to_s
    username
  end

  def self.authenticate(username, password)
    @user = User.find_by(username: username&.downcase)
    return nil if @user.nil?
    return @user if User.hash_password(password, @user.p_salt) == @user.p_hash

    nil
  end

  def password=(passwd)
    @password = passwd
    return if passwd.blank?

    self.p_salt = User.salt
    self.p_hash = User.hash_password(@password, p_salt)
  end

  def self.salt
    BCrypt::Engine.generate_salt
  end

  def self.hash_password(password, salt)
    BCrypt::Engine.hash_secret(password, salt)
  end

  private

  def pass_req?
    p_hash.blank? || password.present?
  end

  def downcase_username
    self.username = username.downcase
  end
end
