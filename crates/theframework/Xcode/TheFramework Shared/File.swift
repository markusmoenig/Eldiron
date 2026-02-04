//
//  File.swift
//  Eldiron
//
//  Created by Markus Moenig on 19/1/23.
//

import Foundation

class File : NSObject
{
    var containerUrl: URL? {
        return FileManager.default.url(forUbiquityContainerIdentifier: nil)?.appendingPathComponent("Documents")
    }
    
    override init()
    {
        super.init()
        
        //print(containerUrl)
        // --- Check for iCloud container existence
        if let url = self.containerUrl, !FileManager.default.fileExists(atPath: url.path, isDirectory: nil) {
            do {
                try FileManager.default.createDirectory(at: url, withIntermediateDirectories: true, attributes: nil)
            }
            catch {
                print(error.localizedDescription)
            }
        }
        
        //checkIfExamplesExist()
    }
    
    /*
    func checkIfExamplesExist()
    {
        let url = containerUrl?.appendingPathComponent("Examples")
        var isDir : ObjCBool = false
        if let url = url {
            if FileManager.default.fileExists(atPath: url.path, isDirectory:&isDir) == false {
                print("Examples do not exist")
                
                do {
                    try FileManager.default.createDirectory(at: url, withIntermediateDirectories: true, attributes: nil)
                    
                    print("created examples folder")
                }
                catch {
                    print(error.localizedDescription)
                }
            }
            
            copyTemplateToExamples("Bricks", url)
            copyTemplateToExamples("SpaceShooter", url)
            copyTemplateToExamples("JumpAndRun", url)

            /*
            do {
                let mapHelpIndex = try FileManager.default.contentsOfDirectory(atPath: url.path)
                print(mapHelpIndex)
            } catch {
            }*/
        }
    }
    
    func copyTemplateToExamples(_ name: String,_ url: URL)
    {
        guard let path = Bundle.main.path(forResource: name, ofType: "denrim", inDirectory: "Templates") else {
            return
        }
        
        do {
            if let templateData = NSData(contentsOfFile: path) {
                let fileURL = url.appendingPathComponent(name + ".denrim")
                try templateData.write(to: fileURL)
            }
        } catch {
        }
        
        /*
        if let str = try? String(contentsOfFile: path, encoding: String.Encoding.utf8) {
            let fileURL = url.appendingPathComponent(name + ".denrim")
            do {
                try str.write(to: fileURL, atomically: true, encoding: String.Encoding.utf8)
            } catch {
            }
        }*/
    }
    
    func loadTemplate(_ name: String) -> String?
    {
        guard let path = Bundle.main.path(forResource: name, ofType: "denrim", inDirectory: "Templates") else {
            return nil
        }
        
        if let templateData = NSData(contentsOfFile: path) {
            let string = String(decoding: templateData, as: UTF8.self)
            return string
        }
        
        return nil
    }*/
}
