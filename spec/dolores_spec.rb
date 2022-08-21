require 'bundler'
Bundler.require(:default)
require_relative '../doloresmessages.rb'
RSpec.describe "Dolores Messages" do
  describe "#DoloresSundayCheck" do
    it 'responds that Dolores is doing the work of the Lord on Sunday' do
      # event = double("event", :respond => "I'm sorry, I'm doing the Lord\'s work today")
      eventresponse = double()
      allow(Time).to receive (:aweraweraw) { 'i am an arbitrary value' }
      logger = double("logger", :info => nil)
      result = DoloresSundayCheck()
      
      expect(result).to eq "It is not a Sunday"
    end
  end
end

